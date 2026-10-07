# FDX_GetDeviceInfo

**En clair** : demande à l'instrument connecté sa carte d'identité (n° de série, micrologiciel, adresse réseau, code produit, date de mise en service). Ces valeurs alimentent la provenance de chaque mesure (ADR 0005).

## Signature

```c
int32_t __stdcall FDX_GetDeviceInfo(FDX_DeviceInfo *out); /* x86 : ret 4 ; 40 octets */
```

## Ce qui est confirmé

- Un seul argument : un pointeur vers une structure de **40 octets** que la DLL remplit (SDK Mac 1.0.5 et 1.1.0 non dépouillés ; mêmes écritures dans la DLL Windows 1.0.1 x86).
- Pointeur nul : erreur -9992.
- **Ne répond que dans l'état interne 1** ; sinon erreur -9986 et journal « can not use. now status:%d ». Il faut donc être connecté et au repos.
- Contenu :

  | Position | Taille | Contenu | Niveau |
  |---|---|---|---|
  | `+0x00` | u32 | **n° de série**, 8 chiffres ; la DLL le convertit elle-même depuis le BCD des données usine | confirmé (affiché `%08d` par MY-CT1, recoupé avec la détection USB) |
  | `+0x04`, `+0x08`, `+0x0C` | 3 × u32 | **version du micrologiciel** : majeur, mineur, révision, affichée `%u.%02d.%04d` | confirmé |
  | `+0x10` | u32 | n° de série BCD lu dans les données usine à `+0xA538` | position confirmée ; sens divergent selon les sources : adaptateur de lumière ambiante, ou référence utilisateur |
  | `+0x14` | u32 | n° de série BCD lu à `+0xA5D4` | position confirmée ; sens divergent : plaque blanche, ou adaptateur d'irradiance |
  | `+0x18..+0x1D` | 6 octets | **adresse MAC** | confirmé |
  | `+0x1E` | 4 caractères | **code produit**, `ACJ1` ou `9C1D` | copie confirmée ; sens très probable (mêmes préfixes que le n° de série USB) |
  | `+0x22` | 2 octets | remplissage | supposé |
  | `+0x24` | u32 | **date initiale** AAAAMMJJ ; 20190101 = jamais posée (voir `FDX_Connect`) | confirmé |

## Pour le pont

- Allouer un tampon plus large que 40 octets, rempli de zéros : marge de sécurité si une autre version de la DLL écrivait plus loin.
- Archiver aussi les octets bruts, pour pouvoir réinterpréter plus tard.
- Dans la provenance : n° de série, micrologiciel, code produit et date initiale sont **confirmés** ; `+0x10` et `+0x14` restent **inconnus** quant à leur sens.
- Appeler juste après `FDX_Connect`, avant l'étalonnage.

## Vérifié sur l'instrument

Le 7 octobre 2026, MYIRO-1 en USB, DLL 1.0.1.0 x64 (test `palier_connexion_avec_le_vrai_instrument`) ; les valeurs d'identification de l'instrument du poste restent dans l'inventaire local :

- `+0` donne le n° de série connu par Windows et par la détection : **confirmé**.
- Micrologiciel au format `1.02.0005` : 1, 2, 5 dans les trois mots : **confirmé**.
- Adresse MAC au préfixe Konica Minolta `00:20:6B` : **confirmé**.
- Code produit `9C1D` : **confirmé** (l'un des deux préfixes attendus).
- Date initiale déjà posée (2021), différente de 20190101 : la connexion n'a rien écrit.
- `+0x10` contient **la même valeur que le n° de série** et `+0x14` vaut zéro. Observation compatible avec un identifiant par défaut (référence utilisateur ou adaptateur non enregistré) ; le sens reste inconnu.
- Les 40 octets sont archivés bruts par le pont.

## Preuves (locales)

- `fdx-x86/exports/FDX_GetDeviceInfo.asm.txt` : pointeur nul à `0x10036a91`, `ret 4` à `0x10036bd6`.
- `fdx-x86/internes/GetDeviceInfo.asm.txt` : contrôle de l'état 1 à `0x1001e318`, refus à `0x1001e34b`, écriture de `+0x24` à `0x1001e5db`.
- `preuves/myct1/infos-instrument.asm.txt` : formats d'affichage à `0x4026a3` et `0x4026e1`.
- `retroanalyse/logiciels/my-ct1.md` § 5.3 et `retroanalyse/logiciels/myiro-tools.md` § 2 : découpe sur 40 octets d'après les SDK Mac.
