# FDX_Connect

**En clair** : ouvre la session avec l'instrument choisi dans la liste de `FDX_GetDevicePortList`. C'est le palier Connexion, et le premier appel qui parle réellement à l'appareil. **Il peut écrire une fois dans l'instrument** (voir plus bas).

## Signature

```c
int32_t __stdcall FDX_Connect(const FDX_PortInfo *port,   /* une entrée de la liste */
                              uint32_t delai_secondes);   /* 1..60 ; x86 : ret 8 */
```

## Ce qui est confirmé

- Deux arguments : un pointeur vers une entrée de 44 octets, puis un délai.
- Pointeur nul : erreur -9992.
- Premier champ de l'entrée différent de 0 (réseau) et de 1 (USB) : erreur -9992.
- Délai égal à 0 ou supérieur à 60 : erreur -9992.
- **Le délai est en secondes** : le SDK le multiplie par 1000 pour ses temporisations, et l'applique ensuite à toutes les commandes envoyées à l'instrument (SDK Mac 1.0.5 et 1.1.0 non dépouillés ; même contrôle 1..60 sous Windows). EIZO, MY-CT1 et MYIRO tools passent tous **10**.
- En réseau, la connexion passe par TCP, port 8823.
- **Écriture possible dans l'instrument** : pendant la connexion, le SDK lit la « date initiale » de l'appareil (format AAAAMMJJ). Si elle vaut encore la valeur d'usine **20190101**, il y écrit la date du jour de l'ordinateur, puis la relit. Ce comportement est lu dans le SDK Mac (`CFDX::checkSetttingInitialDate`) ; les mêmes chaînes et fonctions existent dans les DLL Windows 1.0.1 et 1.0.3. Tous les logiciels officiels déclenchent donc cette écriture à leur première connexion.
- **Le bit de valeur 4 du code de retour signale un échec de ce contrôle de date** (lecture impossible, écriture refusée, horloge de l'ordinateur antérieure à 2019) : la DLL renvoie alors `code | 4`. Ce n'est ni un succès ni un état de l'instrument. Seul le signe du code indique l'échec de la connexion, comme le fait MY-CT1.

## Ce qui est supposé

- La date initiale sert de date de mise en service (garantie, suivi d'étalonnage) ; elle n'a aucun effet sur la mesure.

## Pour le pont

- Passer l'entrée de 44 octets reçue de la détection, sans la modifier.
- Passer 10 secondes.
- Juger le résultat sur le signe ; journaliser le code brut complet et signaler le bit 4 comme « anomalie de date initiale ».
- Avant la première connexion d'un instrument, vérifier que l'horloge de l'ordinateur est juste : c'est elle qui serait inscrite.
- Lire la date initiale dans `FDX_GetDeviceInfo` (`+0x24`) juste après, et l'archiver.
- Appeler `FDX_RegisterDeviceEventHandler` avant, comme EIZO, pour ne manquer aucun événement (ordre observé chez l'appelant, non exigé par la DLL).

## À vérifier sur l'instrument

- La date initiale lue après connexion (attendue : déjà posée, l'instrument du poste ayant été connecté par MY-CT1 et Ergosoft).
- La valeur du code de retour en cas de succès.

## Preuves (locales)

- `fdx-x86/exports/FDX_Connect.asm.txt` : pointeur nul à `0x100355f3`, champ 0/1 à `0x10035604..0x1003560d`, bornes 1..60 à `0x10035621..0x1003562f`, bit 4 à `0x10035749..0x10035767`, `ret 8`.
- `retroanalyse/logiciels/my-ct1.md` § 7.1 : contrôle et écriture de la date initiale (SDK Mac 1.1.0, `0x3613e`, écriture `0x36256`) ; § 5 : délai en secondes, port TCP 8823.
- `retroanalyse/logiciels/myiro-tools.md` : même logique dans le SDK Mac 1.0.5 ; MYIRO tools passe 10.
- `preuves/eizo-x64/connexion.asm.txt` : valeur 10 à `0x180042b7e`.
