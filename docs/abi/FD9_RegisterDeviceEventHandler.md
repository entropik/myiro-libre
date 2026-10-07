# FD9_RegisterDeviceEventHandler

**En clair** : donne à la DLL la fonction du pont qu'elle appellera pour annoncer ce qui se passe dans l'instrument (papier engagé, mesure en cours, capot ouvert…). **Il faut le faire avant la connexion** : une fois connecté, la DLL refuse.

## Signature

```c
typedef void (__cdecl *FD9_Rappel)(uint32_t etat,      /* FD9_eSDKStatus */
                                   uint32_t travail,   /* n° de travail */
                                   uint32_t page,
                                   uint32_t mesurees); /* plages mesurées */
int32_t __stdcall FD9_RegisterDeviceEventHandler(FD9_Rappel rappel);  /* x86 : ret 4 */
```

## Ce qui est confirmé

- Un argument : l'adresse du rappel, gardée telle quelle. **Un pointeur nul désinscrit** : FD-S2w le fait après un échec de connexion.
- **Refus pendant une connexion** : si une session est ouverte, rien n'est enregistré et le code interne 8055 est renvoyé, public **1103** (`fd9-x86` `0x100b153a..0x100b1549`).
- Le rappel est appelé en **`__cdecl`** avec quatre entiers de 32 bits : la DLL dépile elle-même après l'appel (`fd9-x86` `0x100880a6..0x100880a8`). En Rust : `extern "C" fn(u32, u32, u32, u32)` (`GestionnaireEvenements` dans `crates/fd9-sys`).
- Les noms des quatre arguments viennent du symbole de la bibliothèque Mac. Les valeurs de `etat` sont tirées de sa table de noms : 0 et 1 attente de prise de main, 2 à 11 étapes d'une mesure (entraînement du papier, image, reconnaissance des plages, mesure, nombre de plages), 101 à 103 capot ouvert, bourrage, communication, 201 à 206 erreurs de mesure, 301 erreur fatale.

## Ce qui est supposé

- Le rappel est appelé depuis un fil d'exécution de la DLL, pas celui du pont : la connexion crée un objet qui reçoit l'adresse du rappel, et la DLL importe `CreateThread`. Le rappel doit donc être court, ne jamais rappeler la DLL, et transmettre l'événement au fil principal.
- Les états 0 (`WAIT_NO_AUTHORITY`) et 1 (`WAIT_AUTHORITY`) décrivent la **prise de main** : un seul logiciel à la fois pilote l'instrument réseau.

## Pour le pont

- Enregistrer le rappel avant `FD9_Connect`, comme FD-S2w ; le désinscrire (pointeur nul) après un échec de connexion ou après `FD9_Disconnect`.
- Le rappel ne fait que copier les quatre nombres dans une file ; le journal de session les garde bruts.

## À vérifier sur l'instrument

- Les événements reçus pendant une connexion seule, sans travail ni mesure.

## Preuves (locales)

- `fd9-x86/exports/FD9_RegisterDeviceEventHandler.asm.txt` (refus, enregistrement, `ret 4`).
- Appel du rappel : `0x10088097..0x100880ae` (balayage `desassemble.py fd9-x86 --address 0x10088097 --end 0x100880b1`).
- `retroanalyse/logiciels/fd-s2w.md` § 3 (symbole Mac), § 4.4 (`FD9_eSDKStatus`), § 5 (ordre des appels de FD-S2w).
