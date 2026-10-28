# FD9_SetOption

**En clair** : choisit comment la DLL calcule les données d'une plage (condition de mesure M0, M1 ou M2, illuminant, observateur, type de données rendu). **Elle ne parle pas à l'instrument** : elle range un nombre dans une des quatre variables de la DLL, et c'est tout. C'est la seule fonction `FD9_Set*` admise dans la liste blanche ; cette fiche en donne la preuve.

## Signature

```c
int32_t __stdcall FD9_SetOption(const uint32_t *cle,      /* 0 à 3 */
                                const uint32_t *valeur);  /* x86 : ret 8 */
```

## Ce qui est confirmé

### Aucun échange avec l'instrument

- L'export (`fd9-x86` `0x100b1af0`) n'appelle que trois fonctions : la fonction de travail `0x1008c5b0` (`0x100b1afb`), la fonction de regroupement des codes `0x100b43b0` (`0x100b1b0d`), commune à tous les exports, et, seulement si le journal du SDK est actif, l'écriture du journal `0x100a2dc0` (`0x100b1b2d`).
- La fonction de travail `0x1008c5b0..0x1008c652` **ne contient aucune instruction d'appel** : ni commande envoyée, ni fil lancé, ni attente. Elle ne lit pas le drapeau de session : elle marche de la même façon avec ou sans instrument connecté.
- Elle lit la clé, puis saute par une table de quatre adresses (`0x1008c658` : `0x1008c5ca`, `0x1008c5ea`, `0x1008c60a`, `0x1008c62a`, lues dans le fichier). Chaque branche lit la valeur, la compare à sa borne, puis **écrit une seule variable de la DLL**, ou rend un code de refus :

| Clé | Variable écrite (`fd9-x86`) | Écriture | Valeurs admises | Code interne de refus |
|---|---|---|---|---|
| 0 | `0x1022234c` | `0x1008c5df` | 0 à 15 (`< 16`) | `0x1f41` (8001) |
| 1 | `0x10222350` | `0x1008c5ff` | 0 à 15 (`< 16`) | `0x1f42` (8002) |
| 2 | `0x10222354` | `0x1008c61f` | 0 ou 1 (`< 2`) | `0x1f43` (8003) |
| 3 | `0x10222358` | `0x1008c63f` | 0 à 14 (`< 15`) | `0x1f44` (8004) |
| plus de 3 | rien | — | — | `0x1f45` (8005), `0x1008c64a` |

- **Refus : public 1001.** L'export remplace tout code interne non nul par 1001 avant le regroupement (`0x100b1b02..0x100b1b09`). `FD9_GetLastError` rend donc aussi 1001 : les codes 8001 à 8005 ne sortent jamais de la DLL.
- **Aucun contrôle de pointeur nul**, ni dans l'export ni dans la fonction de travail : la clé est lue en `0x1008c5b8`, la valeur dans chaque branche. Un pointeur nul fait planter le processus.
- La DLL x64 fait exactement la même chose : export `0x1800d1350` → `0x1800a5d80`, mêmes bornes, mêmes codes, quatre variables `0x180265c50`, `0x180265c54`, `0x180265c58`, `0x180265c5c`, aucun appel dans la fonction de travail.

### Qui relit ces variables

- [FD9_GetData](FD9_GetData.md) lit la clé 3 pour choisir entre données brutes et calcul (`0x1008c86c`).
- [FD9_CalculateData](FD9_CalculateData.md), et `FD9_GetData` quand il calcule, lisent les quatre (`0x1008cb86` clé 0, `0x1008cc44` clé 1, `0x1008cc69` clé 2, `0x1008cd90` et `0x1008ce89` clé 3). Pour les clés 0 et 1, la valeur 15 est remplacée par une autre valeur interne avant le calcul (`0x1008cbd4`, `0x1008cc57`).
- La clé 0, après ce remplacement, est passée à la conversion des données brutes en spectre (`0x1008ce02`, fonction `_CalcVirRadianceFactor` de `DIColor.dll`) : c'est bien la condition de mesure.

## Les quatre clés

| Clé | Rôle | Valeurs utiles au pont | Niveau |
|---|---|---|---|
| 0 | condition de mesure | M0 = 0, M1 = 2, M2 = 15 | rôle confirmé (passée à la conversion des données brutes) ; valeurs relevées dans les tables de FD-S2w, **supposées** |
| 1 | illuminant du calcul colorimétrique | D50 = 2 | table de FD-S2w ; la valeur 16 (« Auto » de FD-S2w) est refusée ici |
| 2 | observateur | 2° = 0, 10° = 1 | table de FD-S2w ; borne confirmée |
| 3 | type de données rendu | L\*a\*b\* = 0, spectre = 0x0b (11), données brutes = 0x0c (12) | valeurs relevées dans FD-S2w et dans la séquence de MYIROtools ; longueurs confirmées dans [FD9_CalculateData](FD9_CalculateData.md) |

## Propriété de la mémoire

- Les deux pointeurs appartiennent à l'appelant ; la DLL lit les deux nombres pendant l'appel et ne garde pas les adresses (confirmé : registres seulement).
- Le réglage, lui, reste dans la DLL, pour tout le processus, jusqu'au prochain `FD9_SetOption` sur la même clé (confirmé : variables globales). Il n'est pas remis à zéro par une autre fonction lue pour ces fiches (supposé pour les fonctions non lues).

## Ce qui est supposé

- Le sens exact des valeurs de la clé 0 (M0 = 0, M1 = 2, M2 = 15), tiré des tables de FD-S2w.
- Les réglages survivent à `FD9_Disconnect` (aucune écriture de ces adresses vue dans la fermeture).

## Pour le pont

- Toujours passer deux pointeurs valides.
- Reposer les quatre clés avant chaque lecture ou calcul, sans compter sur un réglage laissé par un appel précédent.
- Un retour autre que 0 arrête la mesure en le disant : la valeur était hors bornes.
- `FD9_SetOption` reste la seule `FD9_Set*` de la liste blanche ; `FD9_SetNetworkSetting` en reste exclu.

## À vérifier sur l'instrument

- Rien côté instrument : la fonction ne lui parle pas.
- Le sens des conditions M0, M1, M2 : comparer les spectres calculés à un export de MYIROtools pour la même feuille (essai E4 du plan).

## Preuves (locales)

- `fd9-x86/exports/FD9_SetOption.asm.txt` (export, remplacement par 1001, `ret 8`) ; `fd9-x64/exports/FD9_SetOption.asm.txt`.
- Fonction de travail : `desassemble.py fd9-x86 --address 0x1008c5b0 --end 0x1008c658` (balayage linéaire, la table de saut n'étant pas suivie par le parcours) ; table `0x1008c658`, 4 mots de 32 bits lus dans le fichier ; `fd9-x64 --address 0x1800a5d80`.
- Lecteurs : `--address 0x1008c6e0` (`FD9_GetData`), `--address 0x1008c980` (calcul).
- Valeurs des clés : `retroanalyse/logiciels/fd-s2w.md` § 4.5 ; `retroanalyse/logiciels/fd9-feuille.md` § 3 (lecture d'une plage).
