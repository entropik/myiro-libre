# Export et import CGATS.17 (`myiro-libre/cgats/1`)

**En clair** : CGATS.17 est un fichier texte que lisent les logiciels de profilage et de contrôle. myiro-libre y écrit une mesure pour qu'elle ne reste jamais enfermée dans la bibliothèque (ADR 0001), et sait relire un tel fichier, écrit par lui ou par un autre logiciel. Ce qu'un fichier ne dit pas reste **inconnu** : rien n'est deviné ni rempli par une valeur par défaut.

Le code est dans la crate `crates/cgats`, indépendante de l'application, de Windows et des ponts : c'est un format d'échange partagé avec le futur RIP (ADR 0004). Fonctions publiques : `ecrire` (mesure → texte) et `lire` (texte → mesure importée).

## Ce que myiro-libre écrit

Une mesure (`myiro-libre/mesure/1`, voir [mesure.md](mesure.md)) donne **trois tableaux** dans le même fichier, un par emplacement de spectre de chaque plage (`m0`, `m1`, `m2`). Chaque tableau commence par la ligne `CGATS.17`.

En-tête du premier tableau :

| Mot-clé | Contenu |
|---|---|
| `ORIGINATOR`, `FILE_DESCRIPTOR` | `myiro-libre`, `Mesure myiro-libre` |
| `CREATED` | heure de la mesure, telle que le pont l'a écrite (avec fuseau) |
| `INSTRUMENTATION`, `SERIAL` | modèle et numéro de série de la provenance |
| `MYIRO_LIBRE_FORMAT` | `myiro-libre/cgats/1` |
| `MYIRO_LIBRE_*` | toute la provenance, un mot-clé par champ (voir plus bas) |

En-tête de chaque tableau :

| Mot-clé | Écrit quand |
|---|---|
| `MYIRO_LIBRE_SPECTRE` | toujours : `m0`, `m1` ou `m2` |
| `MEASUREMENT_CONDITION` (`M0`, `M1`, `M2`) et `MEASUREMENT_SOURCE` (`A`, `D50`, `UVCUT`) | seulement si le pont a **confirmé** la condition de ce spectre |
| `WEIGHTING_FUNCTION "ILLUMINANT,D50"` | seulement si l'illuminant des Lab est confirmé |
| `WEIGHTING_FUNCTION "OBSERVER,2 degree"` | seulement si l'observateur est confirmé. Aujourd'hui il n'est que supposé : il n'est donc pas écrit ici, mais il l'est, qualifié, dans `MYIRO_LIBRE_CALCUL_DEMANDE_OBSERVATEUR` |

Un mot-clé standard est lu sans réserve par les autres logiciels : on n'y met que ce qui est confirmé. Tout mot-clé hors de la liste de CGATS.17 est déclaré par `KEYWORD` avant usage.

Colonnes : `SAMPLE_ID` (rang de la plage, à partir de 1), `LAB_L`, `LAB_A`, `LAB_B` (le Lab calculé pour ce spectre), puis le spectre en réflectance (1 = blanc parfait, pas en pourcentage) : `nm380`, `nm390`… quand les longueurs d'onde sont connues, sinon `MYIRO_LIBRE_SPECTRE_1`, `MYIRO_LIBRE_SPECTRE_2`… Les nombres sont écrits avec tous les chiffres utiles : la relecture rend exactement les mêmes valeurs. Fins de ligne Windows (`\r\n`), texte en UTF-8.

### La provenance dans l'en-tête

Un mot-clé par champ, préfixé `MYIRO_LIBRE_` : `MODELE`, `NUMERO_SERIE`, `MICROLOGICIEL`, `CODE_PRODUIT`, `VERSION_SDK` (`1.0.1`), `EMPREINTE_DLL`, `VERSION_PONT`, `ARCHITECTURE`, `HORODATAGE`, `ETALONNAGE`, `GEOMETRIE` (`ponctuelle`, `bande 2`, `feuille`), `CALCUL_LIBELLE`, puis `CALCUL_DEMANDE` et `CALCUL_OBSERVE` (`confirmee`, `supposee` ou `inconnue`), chacun suivi, s'il n'est pas inconnu, de `_CONDITIONS`, `_LONGUEURS_ONDE` (`380/10`), `_ILLUMINANT` et `_OBSERVATEUR`.

Une donnée qualifiée s'écrit `inconnue`, `confirmee:<valeur>` ou `supposee:<valeur>`. Exemple fictif :

```
MYIRO_LIBRE_ETALONNAGE	"inconnue"
MYIRO_LIBRE_CALCUL_DEMANDE_CONDITIONS	"confirmee:M0 confirmee:M1 confirmee:M2"
MYIRO_LIBRE_CALCUL_DEMANDE_OBSERVATEUR	"supposee:2_degres"
```

Dans une chaîne, un guillemet s'écrit doublé (`""`). Un texte sur plusieurs lignes ne peut pas s'écrire en CGATS : la mesure est alors refusée à l'export, plutôt que modifiée.

Les **données brutes** (152 par plage pour le MYIRO-1) ne sont pas exportées en CGATS : elles servent au pilote libre (ADR 0006), pas aux logiciels de profilage. La sauvegarde de la bibliothèque, elle, garde tout.

## Ce que myiro-libre lit

`lire` accepte un fichier CGATS.17 d'un ou plusieurs tableaux, quel que soit l'identifiant de la première ligne, avec commentaires `#`, blancs ou tabulations, fins de ligne Windows ou Unix. Le résultat est une **mesure importée** (`MesureImportee`) : ce n'est pas une mesure attestée par un pont, et elle ne se confond jamais avec elle.

Chaque donnée y est qualifiée :

- **confirmée** : écrite telle quelle dans le fichier ;
- **supposée** : déduite d'une convention de logiciel (voir ci-dessous) ;
- **inconnue** : absente du fichier. Une date `CREATED ""` (vide) est inconnue, pas une chaîne vide.

Chaque tableau va dans un emplacement `m0`, `m1` ou `m2`, dans cet ordre de préférence :

1. `MYIRO_LIBRE_SPECTRE` (fichier de myiro-libre) ;
2. `MEASUREMENT_CONDITION` `M0`, `M1` ou `M2` : condition confirmée ;
3. `MEASUREMENT_SOURCE` `A`, `D50` ou `UVCUT` : condition **supposée** M0, M1 ou M2. C'est la convention des exports CGATS du fabricant (un fichier par condition, source `A` pour M0, `D50` pour M1, `UVCUT` pour M2), observée sur des exports réels du poste ; elle correspond à la définition des conditions dans l'ISO 13655, mais aucun document du fabricant ne l'affirme.

Sans aucun des trois, le fichier est **refusé** avec une phrase claire : on ne range pas des valeurs sous une condition devinée.

Dans les colonnes : `SAMPLE_ID` donne l'identifiant de la plage (sinon son rang), `LAB_L`, `LAB_A`, `LAB_B` le Lab, et les colonnes `nmNNN` (ou `MYIRO_LIBRE_SPECTRE_N`) le spectre ; les longueurs d'onde doivent être régulières. Les autres colonnes (densités, RVB, emplacement sur la feuille…) sont ignorées. **Un fichier sans spectre donne des spectres inconnus** : tout ce qui exige un spectre (autre condition de mesure, densité calculée, nouvel illuminant) reste inconnu pour cette mesure. Un emplacement absent du fichier (par exemple M0 et M2 d'un export qui ne contient que M1) reste inconnu lui aussi.

Plusieurs tableaux doivent porter les mêmes plages, dans le même ordre. Un fichier de myiro-libre d'une autre version (`MYIRO_LIBRE_FORMAT` différent) ou dont la provenance est incomplète est refusé : il ne se relit pas à moitié.

Écrire une mesure puis la relire rend exactement ses spectres, ses Lab et sa provenance (test de la crate).

À l'écran, « Importer un fichier CGATS… » range la mesure importée dans la condition d'impression choisie à gauche. La bibliothèque conserve le texte du fichier tel quel, avec le seul nom du fichier (pas son dossier), et le relit à chaque affichage : la colonne de gauche et le cartouche disent « Importée » et le fichier d'origine, et une valeur absente du fichier s'affiche « inconnue ».

## Comparaison avec l'export CGATS de référence du fabricant

Comparé à l'export MYIROtools d'une couleur, archivé sur le poste (non versionné), et à des exports CGATS du FD-9 par le logiciel du fabricant. Mêmes conventions : identifiant `CGATS.17`, tabulations, chaînes entre guillemets, `ORIGINATOR`, `FILE_DESCRIPTOR`, `CREATED`, `INSTRUMENTATION`, `SERIAL`, `MEASUREMENT_SOURCE`, `WEIGHTING_FUNCTION`, `KEYWORD`, `NUMBER_OF_FIELDS`, `NUMBER_OF_SETS`, colonnes `SAMPLE_ID`, `LAB_L`, `LAB_A`, `LAB_B` et spectre en `nmNNN` (réflectance de 0 à 1).

Écarts voulus :

| Référence du fabricant | myiro-libre | Pourquoi |
|---|---|---|
| Une condition de mesure par fichier | Trois tableaux (M0, M1, M2) dans un seul fichier | Une mesure garde tous ses spectres (ADR 0001). Un logiciel qui ne lit que le premier tableau lit M0 |
| `CREATED` peut être vide | Toujours l'heure de la mesure, avec fuseau | La date est dans la provenance |
| `INSTRUMENTATION` avec le nom du fabricant | Modèle seul, tel que la provenance le donne | On n'écrit que ce que le pont a attesté |
| `MEASUREMENT_GEOMETRY "45/0"` | Absent | La géométrie optique n'est pas dans la provenance ; elle n'est pas devinée |
| `MEASUREMENT_SOURCE`, `WEIGHTING_FUNCTION` toujours écrits | Écrits seulement si confirmés ; ajout de `MEASUREMENT_CONDITION` | Un mot-clé standard ne porte rien de supposé |
| Lab arrondis à deux décimales | Tous les chiffres utiles | La relecture rend exactement la mesure |
| Colonnes de valeurs d'origine (CMJN, RVB), densités, `SAMPLE_LOC` | Absentes | Une mesure ponctuelle ou une bande n'a pas encore de mire ; la densité sera calculée à partir du spectre |
| Pas de provenance | Provenance complète en mots-clés `MYIRO_LIBRE_*` déclarés | Aucune mesure sans provenance (ADR 0005) |

## Sauvegarde complète de la bibliothèque

La sauvegarde n'est pas un CGATS : c'est une copie de la base de la bibliothèque, en **un seul fichier SQLite** (`bibliotheque-myiro-libre.sqlite` par défaut), lisible par tout outil SQLite. Elle contient tout : conditions d'impression, instruments, mesures complètes (données brutes comprises). Un fichier déjà présent n'est remplacé qu'une fois la copie terminée.

La restauration remplace **toute** la bibliothèque par la sauvegarde ; l'écran demande d'abord l'accord de l'opérateur. Avant de toucher quoi que ce soit, la sauvegarde est examinée en lecture seule : organisation de la base connue (pas plus récente que le logiciel), tables présentes, liens cohérents, chaque mesure relisible. Au moindre défaut, elle est refusée et la bibliothèque reste telle quelle.
