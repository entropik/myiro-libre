# FDX_GetDeviceInfo

**En clair** : demande à l'instrument connecté sa carte d'identité (numéro, version du micrologiciel, adresse réseau). Ces valeurs alimentent la provenance de chaque mesure (ADR 0005).

## Signature

```c
int32_t __stdcall FDX_GetDeviceInfo(RE_FDX_DeviceInfoUnresolved *out); /* x86 : ret 4 */
/* Taille totale et alignement de la structure : NON établis. */
```

## Ce qui est confirmé

- Un seul argument : un pointeur vers une structure que la DLL remplit.
- Pointeur nul : erreur -9992.
- **Ne répond que dans l'état interne 1** ; sinon erreur -9986 et journal « can not use. now status:%d ». Il faut donc être connecté (et, supposé, au repos : pas pendant une mesure ou un étalonnage).
- Champs observés, avec la façon dont MY-CT1 les affiche :

  | Position | Taille | Affichage MY-CT1 | Sens |
  |---|---|---|---|
  | `+0` | 4 octets | nombre, converti depuis un codage BCD par la DLL | supposé : n° de série |
  | `+4`, `+8`, `+12` | 3 × 4 octets | `"%u.%02d.%04d"` | supposé : version du micrologiciel |
  | `+0x10`, `+0x14` | 2 × 4 octets | non affichés | inconnu |
  | `+0x18..+0x1d` | 6 octets | `"%02X:%02X:%02X:%02X:%02X:%02X"` | supposé : adresse MAC (réseau) |
  | `+0x1e` | 4 octets écrits | non affichés | inconnu |

- La DLL écrit au moins jusqu'à `+0x21` : **la structure fait au moins 34 octets**, et sa taille réelle peut être supérieure.

## Ce qui est supposé

- Le numéro à `+0` est le numéro de série (le poste connaît la série 10002006 par Windows : comparaison à faire).
- Le format `x.yy.zzzz` est la version du micrologiciel.

## Pour le pont

- **Allouer large** (par exemple 256 octets mis à zéro), jamais une structure de 34 octets ou de taille supposée : si la DLL écrit plus loin que prévu, un tampon trop petit corromprait la mémoire du pont.
- Lire seulement les champs ci-dessus ; archiver aussi les octets bruts, pour pouvoir réinterpréter plus tard.
- Dans la provenance : n° de série et micrologiciel marqués **supposés** tant que la vérification sur instrument n'est pas faite ; les champs inconnus restent **inconnus**, jamais 0.
- Appeler juste après `FDX_Connect`, avant l'étalonnage.

## À vérifier sur l'instrument

- Que `+0` donne bien 10002006.
- Que la version lue correspond à celle affichée par MY-CT1 ou l'instrument.
- Jusqu'où la DLL écrit réellement (tampon rempli d'un motif connu avant l'appel, puis comparé) : donne la taille de la structure.

## Preuves (locales)

- `fdx-x86/exports/FDX_GetDeviceInfo.asm.txt` : pointeur nul à `0x10036a91`, `ret 4` à `0x10036bd6`.
- `fdx-x86/internes/GetDeviceInfo.asm.txt` : contrôle de l'état 1 à `0x1001e318`, refus « can not use. now status:%d » à `0x1001e34b`, écritures des champs.
- `preuves/myct1/infos-instrument.asm.txt` : formats d'affichage `"%u.%02d.%04d"` à `0x4026a3` et adresse MAC à `0x4026e1`.
