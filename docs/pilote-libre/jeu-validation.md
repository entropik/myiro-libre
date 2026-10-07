# Jeu de validation « données brutes → spectres »

**En clair** : le futur pilote libre devra refaire, à partir des 152 valeurs brutes que rend le MYIRO-1 pour chaque plage, les spectres M0, M1 et M2 que calcule aujourd'hui la DLL du fabricant (ADR 0006). Pour vérifier qu'il les refait à l'identique, on garde des « paires » : pour une même plage, les données brutes et les trois spectres de la DLL. L'ensemble de ces paires est le **jeu de validation**. Un **banc** compare ensuite un calcul candidat à ce jeu et dit, longueur d'onde par longueur d'onde, de combien il s'écarte.

Le code est dans la crate `crates/jeu-validation`, indépendante de Windows et des ponts.

## D'où viennent les paires

L'outil `extraire-jeu` lit les sorties du pont archivées sur le poste et produit le jeu :

```
cargo run -p jeu-validation --bin extraire-jeu -- <jeu.json> <sortie du pont>...
```

Il accepte deux formes de sortie, reconnues d'après leur premier caractère :

- **CSV des tests sur instrument** (`crates/pont-myiro1/tests/dll.rs`, paliers 4 et 5) : une en-tête `plage;donnees;L;a;b;nm380;…;nm730`, puis pour chaque plage quatre lignes `M0`, `M1`, `M2` (Lab puis 36 valeurs de spectre) et `brutes` (4 champs vides puis 152 valeurs). Ce CSV ne porte aucun identifiant d'instrument.
- **Lignes JSON du protocole** (`pont-protocole`), une réponse par ligne : seules les réponses `mesure` donnent des paires, une par plage ; les autres sont ignorées.

Toute plage incomplète (spectre ou données brutes manquants), en double ou de mauvaise longueur fait échouer l'extraction : rien n'est deviné.

## Identifiants de l'instrument

Le jeu ne contient aucun identifiant de l'instrument :

- le **numéro de série** de la provenance est remplacé par un pseudonyme, `instrument-1`, `instrument-2`… dans l'ordre d'apparition. Deux plages d'un même instrument portent le même pseudonyme : c'est utile, car le calcul dépend des données d'étalonnage d'usine propres à chaque appareil ;
- le code produit, l'adresse MAC, l'adresse IP, le port et les dates ne sont pas repris.

Le jeu contient malgré tout des mesures réelles : il se produit **en local, hors git** (par exemple à côté des sorties, dans `Archivage/donnees/`). Seules des données synthétiques sont versionnées, dans `crates/jeu-validation/tests/donnees/`, avec des numéros fictifs (12345678).

## Format du jeu (`myiro-libre/jeu-validation/1`)

Un fichier JSON :

```json
{
  "format": "myiro-libre/jeu-validation/1",
  "longueurs_onde": [380, 390, "…", 730],
  "paires": [
    {
      "source": "essai2.csv",
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
| `source` | nom du fichier de sortie d'où vient la plage |
| `plage` | nom de la plage dans ce fichier (`papier`, `1A1`…) ; pour les lignes JSON, `mesure-N/plage-K` |
| `instrument` | `null` si la sortie ne le dit pas (CSV) ; sinon `pseudonyme`, `modele`, `micrologiciel`, `version_sdk`, `empreinte_dll` (SHA-256 de la DLL du fabricant), `calcul` (conditions de calcul demandées à la DLL) |
| `brutes` | les 152 données brutes, telles que rendues par la DLL |
| `spectres` | M0, M1, M2 dans cet ordre, facteur de réflexion de 0 à 1 |

## Le banc de comparaison

Un calcul candidat implémente le trait `CalculSpectres` : il reçoit les 152 données brutes et rend les trois spectres. Deux candidats sont prévus : un candidat synthétique dans les tests, puis le pilote libre.

`comparer(&jeu, &candidat)` rend un rapport, pour chacune des conditions M0, M1 et M2 :

- le nombre de paires comparées ;
- l'écart moyen et l'écart maximal, en valeur absolue, sur toutes les paires et toutes les longueurs d'onde ;
- les mêmes écarts **pour chaque longueur d'onde**, pour repérer où un calcul s'écarte (par exemple seulement dans les bleus).

Les plages que le candidat ne sait pas calculer, ou pour lesquelles il rend un spectre de mauvaise longueur, sont listées à part (`echecs`) et ne comptent pas dans les écarts. Sans aucune paire comparée, les écarts sont inconnus (`null`), jamais mis à zéro.

Le critère d'acceptation du pilote libre reste celui de l'ADR 0006 : des résultats **identiques** à ceux de la DLL. Le banc mesure l'écart ; le seuil qui vaudra « identique » (arrondi des nombres à virgule) sera fixé avec les premiers calculs réels.
