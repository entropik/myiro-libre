# FDX_Connect

**En clair** : ouvre la session avec l'instrument choisi dans la liste de `FDX_GetDevicePortList`. C'est le palier Connexion, et le premier appel qui parle réellement à l'appareil.

## Signature

```c
int32_t __stdcall FDX_Connect(const RE_FDX_Port44 *port,          /* une entrée de la liste */
                              uint32_t timeout_unresolved_units); /* 1..60 ; x86 : ret 8 */
```

## Ce qui est confirmé

- Deux arguments : un pointeur vers une entrée de 44 octets, puis un entier.
- Pointeur nul : erreur -9992.
- Premier champ de l'entrée différent de 0 et de 1 : erreur -9992.
- Second argument égal à 0 ou supérieur à 60 : erreur -9992. Valeurs admises : **1 à 60**.
- EIZO passe **10** ; MY-CT1 passe l'entrée choisie par l'utilisateur et un second argument transmis par son propre code.
- **Le code de retour peut porter un bit supplémentaire** : quand un indicateur interne est posé, la DLL renvoie son code habituel avec le bit de valeur 4 ajouté (`code | 4`). Zéro / non zéro ne suffit donc pas à juger du succès : seul le signe compte (négatif = erreur), comme le fait MY-CT1.

## Ce qui est supposé

- Le second argument est un délai d'attente (borné, et EIZO prend une valeur ronde). Son unité, probablement la seconde, n'est pas démontrée.
- Le bit 4 signale un avertissement ou un état particulier de l'instrument ; son sens est inconnu.

## Pour le pont

- Passer l'entrée de 44 octets reçue de la détection, sans la modifier.
- Utiliser 10 comme EIZO, tant que l'unité n'est pas établie.
- Juger le résultat sur le signe seulement, et journaliser le code brut complet (bit 4 compris).
- Appeler `FDX_RegisterDeviceEventHandler` avant, comme EIZO, pour ne manquer aucun événement (ordre observé chez l'appelant, non exigé par la DLL : à confirmer).

## À vérifier sur l'instrument

- Le temps réel avant échec quand l'instrument est débranché, avec 10 : donne l'unité.
- La valeur du code de retour en cas de succès, et dans quels cas le bit 4 apparaît.
- L'état interne après connexion (attendu 1, qui ouvre `FDX_GetDeviceInfo`).

## Preuves (locales)

- `fdx-x86/exports/FDX_Connect.asm.txt` : pointeur nul à `0x100355f3`, champ 0/1 à `0x10035604..0x1003560d`, bornes 1..60 à `0x10035621..0x1003562f`, bit 4 à `0x10035749..0x10035767`, `ret 8`.
- `preuves/eizo-x64/connexion.asm.txt` : valeur 10 à `0x180042b7e`.
- `preuves/myct1/connexion.asm.txt` : entrée choisie (pas de 44 octets) et test du signe à `0x4025b0`.
