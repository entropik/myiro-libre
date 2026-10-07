# FD9_Connect

**En clair** : ouvre la session avec un FD-9, choisi dans la liste de la détection ou désigné directement par son adresse IP. C'est le palier Connexion et le premier appel qui parle à l'instrument. Le pont s'y présente sous un nom d'application. Aucune clé de licence n'est demandée.

## Signature

```c
int32_t __stdcall FD9_Connect(const tFD9_DeviceData *appareil,  /* 44 octets */
                              const char *nom_application);     /* x86 : ret 8 */
```

## Ce qui est confirmé

- Deux arguments : une entrée `tFD9_DeviceData` (voir [FD9_GetDeviceList](FD9_GetDeviceList.md)) et un texte terminé par zéro. **Pas de délai** en argument, contrairement au MYIRO-1.
- Un des deux pointeurs nul : code **1001** (`fd9-x86` `0x100b0f23..0x100b0f39`).
- **Déjà connecté : la fonction rend 0 sans rien faire** (`0x1008a984`, `0x1008abac`). Un second `FD9_Connect` vers un autre instrument « réussit » donc en restant sur le premier.
- Adresse vide : code interne 2002, public 1001. Liaison autre que 0 ou 1 : code interne 2001, public 1001 (`0x1008f1ea..0x1008f2ae`).
- Seuls la liaison et l'adresse sont lus ; **l'identifiant (`+0x24`) est ignoré** (`0x1008a999..0x1008a9bb`).
- **Liaison 0 (réseau)** : connexion TCP au **port 49152**, fixé dans la DLL (`0x10087481`). Selon un test sur le texte (`0x100873f0`, supposé : « est-ce une adresse `a.b.c.d` ? »), l'adresse est convertie directement ou résolue comme nom d'hôte par `gethostbyname`. Échec de connexion TCP : code interne 4001 ; nom introuvable : 4010 ; tous deux publics **1003**.
- **Liaison 1 (USB)** : ouverture du port série nommé dans l'adresse (`COMn`).
- **Nom d'application** : la DLL en recopie au plus **20 octets** dans une commande envoyée à l'instrument, avec une attente de 30 secondes (`0x1008fbd2..0x1008fc2c`). FD-S2w envoie le nom de l'application suivi du nom de l'ordinateur entre parenthèses.
- Si l'instrument refuse cette commande, la DLL produit le code interne 10002108 ou 10002109, absent de la table : public **1999**. Seul `FD9_GetLastError` les distingue.
- Ensuite, selon un état interne, la DLL vérifie l'échéance de la calibration périodique : **1901** veut dire « connecté, calibration périodique due » (code interne 8011, `0x1008ab83..0x1008ab94`). FD-S2w le traite comme un succès, avec un avertissement.
- Une connexion réussie pose le drapeau de session (1 = réseau, 2 = USB).

## Ce qui est supposé

- 10002108 et 10002109 signalent un refus de **prise de main** : l'instrument réseau est déjà piloté par un autre logiciel (FD-S2w resté ouvert, par exemple).
- La commande du nom d'application est celle qui demande la prise de main ; l'instrument affiche peut-être ce nom.

## Paramètres de connexion réseau du FD-9

L'ADR 0005 prévoit un paramètre de connexion propre à chaque SDK. Pour le FD-9, il comprend :

| Paramètre | Valeur | Niveau |
|---|---|---|
| Liaison | 0 (réseau) ou 1 (USB) | confirmé |
| Adresse | IP de l'instrument, par exemple `192.168.1.40`, ou nom d'hôte ; 31 caractères au plus, car la DLL mesure le texte jusqu'au zéro final | confirmé |
| Port | 49152, fixé dans la DLL : rien à régler | confirmé |
| Identifiant | inutile à la connexion ; mis à zéro quand l'adresse est saisie | confirmé |
| Nom d'application | `myiro-libre` ; 19 caractères ASCII au plus, pour que le zéro final tienne dans les 20 octets recopiés | confirmé pour la recopie, choix du nom propre au projet |
| Délai | aucun : la DLL attend 30 secondes la réponse à la commande du nom | confirmé |
| Clé de licence | aucune : FD9SDK n'en lit pas (la licence de FD-S2w concerne le SDK du FD-5 BT) | confirmé (`fd-s2w.md` § 2.4) |

En réseau, **la détection n'est pas obligatoire** : une entrée construite à la main avec la liaison 0 et l'adresse suffit, puisque la DLL ne lit rien d'autre (`Appareil::reseau` dans `crates/fd9-sys`). C'est utile si la diffusion UDP est bloquée ou si FD-S2w occupe le port local. Les réglages réseau de l'instrument lui-même (adresse fixe ou automatique) ne se changent que par `FD9_SetNetworkSetting`, exclu de la liste blanche : ils se règlent avec FD-S2w ou sur l'instrument.

## Pour le pont

- Enregistrer le rappel d'événements avant, comme FD-S2w ([FD9_RegisterDeviceEventHandler](FD9_RegisterDeviceEventHandler.md)).
- Ne jamais appeler `FD9_Connect` sur une session déjà ouverte : le succès serait trompeur.
- Juger le résultat avec `connexion_etablie` (0 ou 1901) ; signaler 1901 à l'opérateur ; journaliser le code public et le code interne de `FD9_GetLastError`.
- Fermer FD-S2w, ou au moins le déconnecter, avant la connexion.
- Lire aussitôt l'identité avec [FD9_GetSystemInfo](FD9_GetSystemInfo.md).

## À vérifier sur l'instrument

- La connexion directe au FD-9 du réseau par son adresse IP, FD-S2w fermé : code 0 ou 1901.
- Le code obtenu quand FD-S2w est resté connecté (prise de main refusée attendue).
- Ce que l'instrument affiche du nom d'application.

## Preuves (locales)

- `fd9-x86/exports/FD9_Connect.asm.txt` (pointeurs, `ret 8`) et `fd9-x64/exports/FD9_Connect.asm.txt`.
- Session : `desassemble.py fd9-x86 --address 0x1008a970` (déjà connecté, drapeau de session, calibration périodique).
- Ouverture du transport : `--address 0x1008f1a0` (adresse vide, choix réseau ou USB) ; TCP : balayage `--address 0x10087471 --end 0x10087599` (port 49152, `gethostbyname`, `connect`).
- Nom d'application : `--address 0x1008fbb0`.
- `retroanalyse/logiciels/fd-s2w.md` § 2.4 (licence), § 5 (séquence et nom passé par FD-S2w, traitement de 1901).
