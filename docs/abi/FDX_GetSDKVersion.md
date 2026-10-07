# FDX_GetSDKVersion

**En clair** : demande à la DLL son numéro de version. Ne parle pas à l'instrument. C'est le premier appel du palier Version : il prouve que la DLL est chargée et répond.

## Signature

```c
int32_t __stdcall FDX_GetSDKVersion(RE_FDX_Version *out);   /* x86 : ret 4 */
typedef struct { uint32_t part0, part1, part2; } RE_FDX_Version; /* 12 octets */
```

## Ce qui est confirmé

- Un seul argument : un pointeur vers trois entiers de 32 bits, que la DLL remplit.
- Pointeur nul : erreur -9992.
- **La version est écrite en dur dans la DLL**, pas lue sur l'instrument :
  - `part0` = 1 ;
  - `part1` = le texte formé par deux chiffres collés (format `"%d%d"`), reconverti en nombre ;
  - `part2` = 0.
- Valeurs effectives :

  | DLL | part0 | part1 | part2 |
  |---|---|---|---|
  | 1.0.1.0 x86 (MY-CT1) | 1 | 1 (texte « 01 ») | 0 |
  | 1.0.1.0 x64 (Ergosoft) | 1 | 1 (texte « 01 ») | 0 |
  | 1.0.3.0 x64 (EIZO) | 1 | 3 (texte « 03 ») | 0 |

- **Refus possible pendant une connexion** : si un instrument est connecté et que l'état interne n'est pas 1, 2 ou 8, l'appel échoue avec -9986 et le journal « device is connected. ». Appelée avant la connexion, comme le prévoit la progression, la fonction ne peut pas tomber dans ce cas.

## Ce qui est supposé

- `part1` regroupe le deuxième et le troisième chiffre de la version commerciale (« 0 » et « 1 » pour 1.0.1). La valeur `{1, 1, 0}` ne permet pas de distinguer 1.0.1 de 1.1.0 : le pont doit archiver les trois nombres bruts, pas une chaîne reconstruite.

## Pour le pont

- Archiver `part0`, `part1`, `part2` tels quels dans la provenance, plus l'empreinte SHA-256 de la DLL, qui identifie la version sans ambiguïté.
- Appeler avant `FDX_Connect`.

## À vérifier sur l'instrument

- Rien de bloquant : la fonction ne communique pas avec l'appareil. Seul reste à observer le code de retour positif ou nul en cas de succès.

## Preuves (locales)

- `fdx-x86/exports/FDX_GetSDKVersion.asm.txt` : contrôle du pointeur à `0x100374a4`, refus « device is connected. » à `0x10037520`, écriture des trois champs à `0x1003767b..0x1003768d`, `ret 4` à `0x100376c4`.
- `fdx-x64-101/exports/FDX_GetSDKVersion.asm.txt` (`0x18003e5bf` : chiffre 1) et `fdx-x64-103/exports/FDX_GetSDKVersion.asm.txt` (`0x18003e65f` : chiffre 3).
- Chaîne `"%d%d"` à `0x100b625c` dans la DLL x86.
