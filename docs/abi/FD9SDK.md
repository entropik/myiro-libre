# Fiches d'ABI : FD9SDK (FD-9)

Une fiche par fonction de `FD9SDK.dll` utilisée par les paliers Version, Détection et Connexion du FD-9. Elles sont établies par lecture statique de la DLL Windows (x86 et x64), de la bibliothèque Mac de FD-S2w, qui a gardé les noms de ses fonctions internes et leurs types, et des appels de FD-S2w. Aucune DLL n'a été chargée et aucun instrument n'a été interrogé pour les écrire.

Les niveaux sont ceux des fiches du MYIRO-1 ([README](README.md)) :

- **confirmé** : lu directement dans le code du SDK, avec l'adresse citée ;
- **supposé** : déduction plausible, non démontrée ;
- **à vérifier sur l'instrument** : seule une exécution réelle peut trancher.

Les preuves citées (`Audit-MYIRO/retroanalyse/…`) sont des désassemblages de binaires propriétaires : elles restent sur le poste et ne sont pas publiées. Les fiches ne recopient pas de code du fabricant. Les adresses sont des adresses préférées de module (`VA`).

Les désassemblages des exports se régénèrent avec `python -I Audit-MYIRO/outils/desassemble.py fd9-x86 fd9-x64` (les exports `JIG_*` sont sautés), et une fonction interne avec `--address`.

## Versions étudiées

| Repère | Fichier | Version | Architecture |
|---|---|---|---|
| `fd9-x86` | FD-S2w 1.6.0 `Module/FD9SDK.dll` | 1.3.2.3 | x86 |
| `fd9-x64` | Ergosoft `FD9SDK.dll` | 1.3.1.5 | x64 |
| FD-S2w 1.6.1 Mac | `libFD9SDK.dylib` | 1.3.2.3 | x86_64, **symboles conservés** |

Les deux DLL Windows ont les mêmes 125 exports : 38 fonctions `FD9_*`, 86 fonctions `JIG_*` (maintenance usine et tests de l'appareil), jamais appelées, et une donnée, `gSDKLog` (variable du journal du SDK, rôle supposé d'après son nom), qui n'est pas une fonction et n'est jamais résolue par le pont.

FD9SDK est un SDK distinct de FDXSDK : noms, codes d'erreur, structures et transport réseau diffèrent. Le nom de code interne du FD-9 est « Jungfrau ».

## Règles communes

- **Convention d'appel x86 : `__stdcall`** pour les exports de ces fiches (confirmé : `ret N` égal à 4 octets par argument). Deux exports hors de ces fiches sont en `__cdecl` (`FD9_GetPremeasurementError`, `FD9_WriteBmpFile`). Le rappel d'événements est appelé en `__cdecl`.
- **Les scalaires d'entrée passent par pointeur** (confirmé) : par exemple, la capacité de `FD9_GetDeviceList` est un `const uint32_t *`. Le code C++ d'origine utilise des références.
- **Session globale** (confirmé) : aucune poignée d'instrument ; un seul instrument à la fois par processus. Un drapeau interne vaut 0 hors connexion, 1 connecté en réseau, 2 connecté en USB (`fd9-x86` `0x1008aa3c..0x1008aa44`).
- **Retour : un code public regroupé**, 0 = succès (confirmé). Chaque export passe son code interne à une fonction de regroupement (`fd9-x86` `0x100b43b0`) qui le cherche dans une table de 32 groupes (`0x10108ca0`, identique dans la bibliothèque Mac) et renvoie le numéro du groupe ; un code absent de la table devient **1999**.
- **Propriété de la mémoire** : chaque fiche a sa rubrique. En résumé, pour ces six exports, la DLL ne garde aucun pointeur de l'appelant après le retour, **sauf le rappel d'événements**, relu à chaque événement.
- **`FD9_GetLastError` rend le code interne d'origine**, pas le code regroupé (confirmé en x86 et en x64). C'est le seul moyen de distinguer deux échecs du même groupe.

## Codes publics rencontrés dans ces fiches

| Code | Sens | Codes internes d'origine relevés | Niveau |
|---|---|---|---|
| 0 | succès | — | confirmé |
| 1001 | paramètre invalide | 1001 (contrôles des exports), 2001 (liaison inconnue), 2002 (adresse vide) | valeurs confirmées |
| 1002 | échec réseau pendant la détection | 12051 à 12057 (sockets de la détection) | valeurs confirmées, sens supposé |
| 1003 | instrument injoignable | 4001 (échec de la connexion TCP), 4010 (échec de la résolution du nom) | valeurs confirmées, sens supposé d'après l'appel qui précède |
| 1004 | aucun instrument connecté | 8054 | valeurs confirmées, sens tiré du code de `FD9_Disconnect` |
| 1103 | appel interdit pendant une connexion | 8055 | valeurs confirmées, sens tiré du code |
| 1104 | appel interdit pendant une mesure | 8064 | valeurs confirmées, sens supposé |
| 1901 | **connecté**, mais calibration périodique due | 8011 | confirmé ; FD-S2w le traite comme un succès |
| 1999 | code interne absent de la table | par exemple 10002108, 10002109 (voir `FD9_Connect`) | confirmé |

Les autres groupes (1005 à 1010, 1101 à 1113, 1201 à 1209, 1902) existent mais leur sens n'est pas établi.

## Transport

- **USB** (confirmé) : port série virtuel (`COMn`). L'instrument est retenu si son identifiant Windows commence par `USB\VID_132B&PID_210D\` suivi de `9C1A` ou `A8AN`. Supposé : les deux préfixes distinguent FD-9 et MYIRO-9.
- **Réseau** (confirmé) : détection par diffusion UDP, commandes en TCP. Les deux utilisent le **port 49152**. Voir [FD9_GetDeviceList](FD9_GetDeviceList.md) et [FD9_Connect](FD9_Connect.md).
- **Journal du SDK** (supposé pour Windows, confirmé sur Mac) : si un dossier `DebugSDK` ou `DebugImage` existe dans le dossier courant du pont, le SDK y écrit un journal horodaté de chaque appel. Le pont ne doit pas créer ces dossiers sans le vouloir ; c'est en revanche un outil de diagnostic utile pour le premier essai.

## Fiches

1. [FD9_GetLastError](FD9_GetLastError.md) : dernier code interne ; seul appel du palier Version
2. [FD9_GetDeviceList](FD9_GetDeviceList.md) : instruments visibles en USB et sur le réseau (palier Détection)
3. [FD9_RegisterDeviceEventHandler](FD9_RegisterDeviceEventHandler.md) : rappel d'événements, à poser avant la connexion
4. [FD9_Connect](FD9_Connect.md) : ouverture de la session et **paramètres de connexion réseau** (palier Connexion)
5. [FD9_GetSystemInfo](FD9_GetSystemInfo.md) : identité de l'instrument et version du SDK (palier Connexion)
6. [FD9_Disconnect](FD9_Disconnect.md) : fermeture de la session

Ces six exports sont les seuls de la liste blanche de `crates/fd9-sys`. `FD9_TAConnect` (connexion prioritaire), `FD9_SetOption`, `FD9_SetNetworkSetting`, `FD9_RegisterUserIlluminant` et tous les `JIG_*` en restent exclus.

## Preuves (locales)

- `retroanalyse/fd9-x86/` et `retroanalyse/fd9-x64/` : `metadata.json` (exports, imports) et `exports/*.asm.txt`.
- `retroanalyse/logiciels/fd-s2w.md` : étude de FD-S2w 1.6.0 et 1.6.1, signatures tirées des symboles Mac (§ 3 et 4), séquence de FD-S2w (§ 5), table des codes (§ 6), transport (§ 7).
