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

## Contenu d'une entrée `tFD9_DeviceData` (44 octets en 32 et 64 bits)

| Position | Taille | Contenu | Niveau |
|---|---|---|---|
| `+0x00` | u32 | liaison : **0 = réseau (TCP), 1 = USB** | confirmé |
| `+0x04` | 32 octets | `COMn` en USB, adresse IP `a.b.c.d` en réseau, terminée par zéro | confirmé |
| `+0x24` | 8 octets | identifiant, sans zéro final : n° de série USB sans ses 4 premiers caractères, ou 4 octets de la réponse réseau en hexadécimal | contenu confirmé ; sens de la valeur réseau supposé (n° de série de l'instrument) |

Taille confirmée par le pas de 44 octets (`imul …, 0x2c`, `0x100ae273`) ; aucune adresse mémoire dans la structure, donc même forme en 32 et 64 bits (`Appareil` dans `crates/fd9-sys`).

## Ce qui est supposé

- Si FD-S2w tourne sur le même ordinateur, il occupe déjà le port local 49152 : la détection réseau échoue alors, sans gêner l'USB.
- Appelée pendant une connexion, la fonction n'est pas refusée par l'export ; son effet sur la session ouverte n'est pas établi.

## Pour le pont

- Passer une capacité de 1 à 100 (FD-S2w passe 10) et un tableau de cette taille.
- Toujours relire le compteur ; ne lire que les entrées comptées.
- Présenter chaque entrée avec sa liaison, son adresse et son identifiant ; rendre les 44 octets intacts à `FD9_Connect`.
- Fermer FD-S2w avant la détection réseau.
- Appeler avant `FD9_Connect`, jamais pendant une session.

## À vérifier sur l'instrument

- La liste avec le FD-9 du réseau : une entrée, liaison 0, son adresse IP.
- Le délai d'attente des réponses réseau (fixé par le SDK, non relevé).

## Preuves (locales)

- `fd9-x86/exports/FD9_GetDeviceList.asm.txt` (contrôles, succès partiel, `ret 0xc`) et `fd9-x64/exports/FD9_GetDeviceList.asm.txt`.
- Recherche : `desassemble.py fd9-x86 --address 0x100ae260` (filtres USB vers `0x100ae372..0x100ae49f`, entrée USB `0x100ae554..0x100ae58f`, diffusion `0x100ae856..0x100ae8f7`, réponse de 24 octets `0x100ae978..0x100aea7c`).
- `retroanalyse/logiciels/fd-s2w.md` § 4.3 (structure, champs vus sur Mac), § 5 (FD-S2w passe 10), § 7 (transport).
