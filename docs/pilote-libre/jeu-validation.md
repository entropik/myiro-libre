# Jeu de validation « données brutes → spectres »

**En clair** : le futur pilote libre devra refaire, à partir des 152 valeurs brutes que rend le MYIRO-1 pour chaque plage, les spectres M0, M1 et M2 que calcule aujourd'hui la DLL du fabricant (ADR 0006). Pour vérifier qu'il les refait à l'identique, on garde des « paires » : pour une même plage, les données brutes et les trois spectres de la DLL. L'ensemble de ces paires est le **jeu de validation**. Un **banc** compare ensuite un calcul candidat à ce jeu et dit, longueur d'onde par longueur d'onde, de combien il s'écarte.

Le code est dans la crate `crates/jeu-validation`, indépendante de Windows et des ponts.

## D'où viennent les paires

L'outil `extraire-jeu` lit les sorties du pont archivées sur le poste et produit le jeu :

```
cargo run -p jeu-validation --bin extraire-jeu -- <jeu.json> <sortie du pont>...
```

Il accepte deux formes de sortie, reconnues d'après leur premier caractère :

- **CSV des tests sur instrument** (`crates/pont-myiro1/tests/dll.rs`, paliers 4 et 5) : une en-tête `plage;donnees;L;a;b;nm380;…;nm730`, puis pour chaque plage quatre lignes `M0`, `M1`, `M2` (Lab puis 36 valeurs de spectre) et `brutes` (3 champs vides à la place de L, a, b, puis 152 valeurs). Ce CSV ne porte aucun identifiant d'instrument.
- **Lignes JSON du protocole** (`pont-protocole`), une réponse par ligne : seules les réponses `mesure` donnent des paires, une par plage ; les autres sont ignorées. Les mesures au [format courant](../formats/mesure.md) comme celles du format initial (pont 0.1.0) sont acceptées ; le champ `calcul` du jeu reprend le libellé du calcul.

L'extraction échoue, sans rien deviner, si une plage est incomplète (spectre ou données brutes manquants), si une donnée est en double, si une plage revient plus loin dans le même fichier, si une liste n'a pas la bonne longueur, ou si une valeur n'est pas un nombre fini (`NaN`, `inf`).

## Identifiants de l'instrument

Le jeu ne contient aucun identifiant de l'instrument :

- le **numéro de série** de la provenance est remplacé par un pseudonyme, `instrument-1`, `instrument-2`… dans l'ordre d'apparition. Deux plages d'un même instrument portent le même pseudonyme : c'est utile, car le calcul dépend des données d'étalonnage d'usine propres à chaque appareil ;
- le code produit, l'adresse MAC, l'adresse IP, le port et les dates ne sont pas repris ;
- le **nom des fichiers** lus, qui peut contenir un numéro de série, n'entre pas dans le jeu : chaque fichier y devient `sortie-1`, `sortie-2`… dans l'ordre de la commande. L'outil affiche la correspondance à l'écran, sans l'enregistrer.

Le jeu contient malgré tout des mesures réelles : il se produit **en local, hors git** (par exemple à côté des sorties, dans `Archivage/donnees/`). Seules des données synthétiques sont versionnées, dans `crates/jeu-validation/tests/donnees/`, avec des numéros fictifs (12345678).

## Format du jeu (`myiro-libre/jeu-validation/1`)

Un fichier JSON :

```json
{
  "format": "myiro-libre/jeu-validation/1",
  "longueurs_onde": [380, 390, "…", 730],
  "paires": [
    {
      "source": "sortie-1",
      "plage": "papier",
      "instrument": null,
      "brutes": ["152 nombres"],
      "spectres": [["36 nombres : M0"], ["36 nombres : M1"], ["36 nombres : M2"]]
    }
  ]
}
```

| Champ | Sens |
|---|---|
| `format` | version du format ; un changement incompatible change ce nom |
| `longueurs_onde` | 36 longueurs d'onde en nm, de 380 à 730 par 10 |
| `source` | numéro d'ordre du fichier de sortie d'où vient la plage (`sortie-1`…), jamais son nom |
| `plage` | nom de la plage dans ce fichier (`papier`, `1A1`…) ; pour les lignes JSON, `mesure-N/plage-K` |
| `instrument` | `null` si la sortie ne le dit pas (CSV) ; sinon `pseudonyme`, `modele`, `micrologiciel`, `version_sdk`, `empreinte_dll` (SHA-256 de la DLL du fabricant), `calcul` (conditions de calcul demandées à la DLL) |
| `brutes` | les 152 données brutes, telles que rendues par la DLL |
| `spectres` | trois spectres, facteur de réflexion de 0 à 1, dans l'ordre M0, M1, M2 (**confirmé** depuis le 7 octobre 2026, voir ci-dessous) |

**Ce qui est supposé.** Le pont demande les trois spectres à la DLL avec le réglage `Illuminant` à 0, 1 puis 2, et les nomme M0, M1, M2 d'après la fiche [`FDX_GetMeasureData`](../abi/FDX_GetMeasureData.md). L'ADR 0005 range cette correspondance parmi les points encore à confirmer sur l'instrument : tant qu'elle ne l'est pas, les noms M0, M1, M2 du jeu sont supposés. L'ordre des trois spectres, lui, est celui dans lequel le pont les a demandés.

**Complément du 7 octobre 2026.** La fiche [`FDX_GetMeasureData`](../abi/FDX_GetMeasureData.md) range désormais cette correspondance parmi ce qui est confirmé sur l'instrument : `Illuminant` 0, 1, 2 donne M0, M1, M2, et l'illuminant des Lab (code 2) est D50. Les noms M0, M1, M2 du jeu sont donc confirmés. Seul l'observateur 2° (code 0) reste supposé.

## Le banc de comparaison

Le banc prend un **calcul candidat** : un programme qui reçoit les 152 données brutes d'une plage et rend les trois spectres. Aujourd'hui, c'est un candidat d'essai inventé pour les tests ; demain, ce sera le pilote libre. Le banc fait calculer chaque plage du jeu par le candidat, puis compare son résultat aux spectres de la DLL, valeur par valeur.

Il rend un **rapport** :

| Champ | Sens |
|---|---|
| `conditions` | trois blocs, un pour M0, un pour M1, un pour M2, détaillés ci-dessous |
| `echecs` | les plages laissées de côté, avec la raison : le candidat n'a pas su les calculer, a rendu un spectre trop court ou trop long, ou une valeur qui n'est pas un nombre fini ; ou bien la plage du jeu elle-même est abîmée (fichier modifié). Chaque échec donne `source`, `plage` et `detail` |

Chaque bloc de condition contient :

| Champ | Sens |
|---|---|
| `condition` | `M0`, `M1` ou `M2` |
| `paires` | nombre de plages réellement comparées (les échecs n'en font pas partie) |
| `ecart_moyen` | moyenne des écarts entre le candidat et la DLL, toutes plages et toutes longueurs d'onde confondues. Un écart est la différence sans signe entre les deux valeurs : 0,01 veut dire un point de réflexion sur cent |
| `ecart_maximal` | le plus grand de ces écarts |
| `par_longueur_onde` | 36 lignes, une par longueur d'onde (`longueur_onde` en nm, `ecart_moyen`, `ecart_maximal`) : elles montrent où le calcul s'écarte, par exemple seulement dans les bleus |

Une plage en échec ne compte jamais comme un écart nul. Si aucune plage n'a pu être comparée, les écarts sont inconnus (`null`), jamais mis à zéro.

Le critère d'acceptation du pilote libre reste celui de l'ADR 0006 : des résultats **identiques** à ceux de la DLL. Le banc mesure l'écart ; le seuil qui vaudra « identique » (arrondi des nombres à virgule) sera fixé avec les premiers calculs réels.
