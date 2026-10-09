# FD9_GetDeviceList

**En clair** : demande à la DLL la liste des FD-9 qu'elle voit, branchés en USB ou présents sur le réseau local. C'est le palier Détection. Une liste vide n'est pas une erreur. En réseau, la détection n'est pas obligatoire : on peut se connecter directement à une adresse IP connue (voir [FD9_Connect](FD9_Connect.md)).

## Signature

```c
int32_t __stdcall FD9_GetDeviceList(tFD9_DeviceData *liste,      /* obligatoire */
                                    uint32_t *trouves,           /* obligatoire */
                                    const uint32_t *capacite);   /* x86 : ret 0xc */
```

## Ce qui est confirmé

- Trois arguments : le tableau à remplir, le compteur, la capacité **passée par pointeur** et comptée en entrées.
- Tableau nul, compteur nul, ou capacité hors de 1 à 100 : code **1001**. Contrairement au MYIRO-1, **il n'y a pas d'appel en deux temps** : le tableau est obligatoire (`fd9-x86` `0x100b0e85..0x100b0e97`, même contrôle en x64).
- La DLL remet le compteur à zéro et efface `capacite` entrées du tableau avant de chercher.
- **USB d'abord** : énumération des ports série Windows, filtre `VID_132B&PID_210D` puis préfixe `A8AN` ou `9C1A` du n° de série USB ; l'entrée reçoit liaison 1, le nom `COMn` et 8 caractères du n° de série.
- **Réseau ensuite** : un message de 16 octets commençant par `FD9SDK` est diffusé vers `255.255.255.255`, port UDP **49152**, depuis le port local 49152. Seules les réponses de 24 octets sont gardées : leurs octets 4 à 7, en hexadécimal, forment l'identifiant, et leurs octets 8 à 11 l'adresse IP, écrite `a.b.c.d`. Une adresse déjà présente n'est pas ajoutée deux fois.
- **Succès partiel** : si une étape échoue mais que le compteur n'est pas nul, la fonction renvoie 0 (`0x100b0ea7..0x100b0eb0`). Un échec réseau (codes internes 12051 à 12057, public 1002) n'empêche donc pas de voir un FD-9 en USB.

## Détection réseau en détail (`fd9-x86`)

Établi le 9 octobre 2026 par lecture de la recherche (`0x100ae5db..0x100ae973`), après une détection réelle qui ne trouvait aucun FD-9.

- **Deux sockets UDP** (confirmé) : une de réception, avec `SO_REUSEADDR`, liée à `0.0.0.0:49152` (`0x100ae699..0x100ae7ac`) ; une d'envoi, avec `SO_BROADCAST`, **non liée** (port local choisi par Windows, `0x100ae7e2..0x100ae7f8`). L'envoi part de la seconde, la réponse n'est lue que sur la première.
- **Un envoi par adresse IPv4 de l'ordinateur** (confirmé) : `gethostbyname("")` donne les adresses locales ; pour chacune, un message de 16 octets (`FD9SDK`, l'adresse locale, une somme de contrôle) est diffusé vers `255.255.255.255:49152`, puis les réponses sont lues jusqu'au premier délai dépassé (`0x100ae853..0x100ae96a`). Les adresses d'un VPN ou de cartes virtuelles reçoivent aussi leur envoi.
- **Délai de lecture : 5 millisecondes** (confirmé pour la valeur passée) : `SO_RCVTIMEO` reçoit une structure de 8 octets `{5, 0}` (`0x100ae6f6..0x100ae71f`) ; Winsock n'en lit que le premier mot, en millisecondes. L'intention était sans doute 5 secondes (supposé). Une réponse plus lente que la suite des envois est perdue à la fermeture des sockets.
- **Pare-feu Windows** (confirmé sur le poste le 9 octobre 2026) : la réponse arrive sur le port 49152 d'une socket qui n'a rien envoyé. L'exception de Windows pour les réponses à une diffusion ne vaut que pour la socket d'envoi : sans règle entrante « UDP, port local 49152 » pour le programme qui charge la DLL, la réponse est bloquée et la liste reste vide, **sans erreur**. Sans règle : liste vide cinq fois de suite. Avec la règle ci-dessous : un FD-9 en réseau trouvé en 0,3 s, liaison 0. Le délai de 5 ms a suffi dans cet essai. Règle, avec des valeurs fictives (droits d'administrateur) :

  ```
  netsh advfirewall firewall add rule name="FD-9 - pont-fd9 - UDP local" dir=in action=allow protocol=UDP localport=49152 remoteip=192.0.2.40 profile=any program="C:\Program Files\myiro-libre\pont-fd9-x86.exe"
  ```

  Le chemin est celui du pont installé ; en développement, celui de `target\i686-pc-windows-msvc\debug\pont-fd9.exe`.
- **Ce que FD-S2w fait de plus** : rien (confirmé, `FD-S2w.exe` `0x41ed50..0x41eda8`) : le même appel, tableau de 10 entrées, capacité 10, sans appel préalable à la DLL. Qu'il trouve le FD-9 sans règle de pare-feu visible n'est pas expliqué (question ouverte).

## Contenu d'une entrée `tFD9_DeviceData` (44 octets en 32 et 64 bits)

| Position | Taille | Contenu | Niveau |
|---|---|---|---|
| `+0x00` | u32 | liaison : **0 = réseau (TCP), 1 = USB** | confirmé |
| `+0x04` | 32 octets | `COMn` en USB, adresse IP `a.b.c.d` en réseau, terminée par zéro | confirmé |
| `+0x24` | 8 octets | identifiant, sans zéro final : n° de série USB sans ses 4 premiers caractères, ou 4 octets de la réponse réseau en hexadécimal | contenu confirmé ; sens de la valeur réseau supposé (n° de série de l'instrument) |

Taille confirmée par le pas de 44 octets (`imul …, 0x2c`, `0x100ae273`) ; aucune adresse mémoire dans la structure, donc même forme en 32 et 64 bits (`Appareil` dans `crates/fd9-sys`).

## Propriété de la mémoire

- Le tableau, le compteur et la capacité appartiennent à l'appelant, qui les alloue (confirmé : la DLL n'alloue rien pour l'appelant et ne rend aucun pointeur).
- La DLL écrit dans le tableau et le compteur **pendant l'appel seulement** : elle ne garde leurs adresses que dans des variables locales de la fonction (`0x100ae280`, `0x100ae292`), et la recherche se fait dans un objet posé sur la pile de l'export (`0x100b0e9c`). Rien n'est gardé après le retour (confirmé).
- Elle écrit au plus `capacite` entrées de 44 octets (effacement initial de `capacite × 44` octets, `0x100ae273..0x100ae298` ; arrêt de la recherche quand le compteur atteint la capacité, en USB `0x100ae340..0x100ae345` et en réseau `0x100ae5d0..0x100ae5d5`, `0x100ae905`). Le tableau doit donc faire au moins `capacite` entrées (confirmé).

## Ce qui est supposé

- Si FD-S2w tourne sur le même ordinateur, il occupe déjà le port local 49152 : la détection réseau échoue alors, sans gêner l'USB.
- Appelée pendant une connexion, la fonction n'est pas refusée par l'export ; son effet sur la session ouverte n'est pas établi.

## Pour le pont

- Passer une capacité de 1 à 100 (FD-S2w passe 10) et un tableau de cette taille.
- Toujours relire le compteur ; ne lire que les entrées comptées.
- Présenter chaque entrée avec sa liaison, son adresse et son identifiant ; rendre les 44 octets intacts à `FD9_Connect`.
- Fermer FD-S2w avant la détection réseau.
- Pare-feu : le pont ne crée aucune règle. L'application demande la sienne à la première détection vide (ADR 0005, complément du 9 octobre 2026, ticket #47).
- Appeler avant `FD9_Connect`, jamais pendant une session.

## À vérifier sur l'instrument

- La liste avec le FD-9 du réseau : **observée le 9 octobre 2026** avec la règle de pare-feu : une entrée, liaison 0 (réseau), son adresse IP et un identifiant de 8 caractères (que ce soit le n° de série reste supposé).
- La marge du délai de 5 ms sur un réseau plus chargé (il a suffi dans l'essai).
- Pourquoi FD-S2w trouve le FD-9 sans règle de pare-feu visible.

## Preuves (locales)

- `fd9-x86/exports/FD9_GetDeviceList.asm.txt` (contrôles, succès partiel, `ret 0xc`) et `fd9-x64/exports/FD9_GetDeviceList.asm.txt`.
- Recherche : `desassemble.py fd9-x86 --address 0x100ae260` (filtres USB vers `0x100ae372..0x100ae49f`, entrée USB `0x100ae554..0x100ae58f`, diffusion `0x100ae856..0x100ae8f7`, réponse de 24 octets `0x100ae978..0x100aea7c`).
- `retroanalyse/logiciels/fd-s2w.md` § 4.3 (structure, champs vus sur Mac), § 5 (FD-S2w passe 10), § 7 (transport).
