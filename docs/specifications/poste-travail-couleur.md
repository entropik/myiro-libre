# Compléter le poste de travail couleur

## Problème et cadrage validé

Le poste de travail couleur doit couvrir la préparation de mires, leur mesure,
le profilage ICC et sa vérification, puis le contrôle des tirages dans le temps.
Ce cadrage résulte du grill validé par le mainteneur le 7 octobre 2026.

Source de suivi : [spécification GitHub #30](https://github.com/entropik/myiro-libre/issues/30)
et tickets #31 à #42, mis à jour et relus le 9 octobre 2026. Cette version
remplace la priorité historique « MYIRO-1 d'abord » et le périmètre initial qui
faisait attendre le profilage après la linéarisation et le contrôle complet.

## Première livraison : profil ICC RVB vérifié

Flux prioritaire hors ligne : mire RVB → impression par l'opérateur → mesure ou
import → profil ICC → seconde impression de vérification → résultats et rapport.
Premier cas terrain : Epson à encres pigmentaires, papier semigloss classique,
via le pilote d'imprimante. Le modèle précis, la référence papier et les réglages
seront renseignés dans la fiche d'essai, sans les inventer.

MYIRO-1 et FD-9 sont développés en parallèle et font tous deux partie de cette
première livraison. Le FD-9 est utilisé quotidiennement et n'est pas reporté
après le MYIRO-1. Les paliers de sécurité matérielle restent applicables.

- Chaque instrument possède ses propres mires et préréglages ; plages plus
  petites possibles au FD-9, dans les limites validées par essais.
- Préréglages rapide, standard et approfondi par instrument, plus mode avancé.
  Dimensions et nombre de plages seront établis par essais.
- L'opérateur imprime lui-même : impression intégrée non requise. Distinguer
  caractérisation sans gestion couleur et vérification à travers le profil.
- Fiche réutilisable confirmée pour chaque tirage : imprimante, papier, encres,
  support choisi dans le pilote, qualité et gestion des couleurs ; capture joignable.
  La version utilisée accompagne mire, mesures et profil.
- Acquisition directe et import, notamment CGATS FD-S2w, alimentent le même parcours.
  Provenance, fichier original et champs inconnus sont conservés.
- Moyennage de plusieurs exemplaires d'une même mire avec le même instrument et
  les mêmes conditions : lectures originales conservées, écarts signalés, aucun
  mélange MYIRO-1/FD-9 ni exclusion automatique.
- Profilage RVB direct sans linéarisation obligatoire.
- Vérification distinguant fidélité aux couleurs demandées et accord avec la
  prédiction du profil ; tolérances réglables/versionnées choisies avant vérification.
- États : non vérifié, accepté selon une version de tolérances, hors tolérance
  ou vérification incomplète. Un profil hors tolérance reste exportable avec son rapport.

## Ordre de livraison

| Tickets | Place dans le cadrage |
|---|---|
| #33 | Mires RVB par instrument, exports et fiche d'impression : première livraison |
| #34 et #35 | FD-9 et MYIRO-1, acquisition/import, reprise et moyennage : première livraison |
| #40 | Profilage RVB direct : première livraison ; CMJN en extension |
| #41 | Vérification ICC et son rapport : première livraison |
| #31 et #32 | Densités, listes de couleurs et nuanciers : ultérieurement |
| #36 et #37 | Contrôle d'impression complet : ultérieurement |
| #38 | Linéarisation et limites d'encre : parcours optionnel ultérieur |
| #39 | Suivi récurrent, rapports de contrôle et étiquettes : ultérieurement |
| #42 | Gamuts : ultérieurement ; priorité au choix entre profils/papiers et à l'explication des différences |

Les fonctions différées restent prévues. Bibliothèque, calculs, tolérances et
rapports nécessaires à #41 appartiennent à la première livraison et n'attendent
pas les nuanciers, les densités ou le contrôle complet.

## Règles transversales

- Sources originales, spectres disponibles et provenance conservés ; absence
  explicite, aucun spectre inventé depuis un Lab.
- Import incomplet conservable et inspectable. Compléments manuels identifiés.
  Association incertaine entre plages et mesures : calcul ICC bloqué ; #1 s'applique.
- Références et tolérances déjà utilisées immuables : nouvelle version pour toute
  correction, nouveau résultat pour un recalcul, anciens rapports reproductibles.
- Critère requis non calculable : contrôle global incomplet. Changer de règle
  exige un choix explicite et produit un résultat distinct.
- Calculs métier indépendants de Tauri, Windows et des ponts ; parcours guidés et reprenables.
- Version, paramètres, commandes et diagnostics ArgyllCMS consultables et archivés.
- Essais matériels soumis aux paliers et accords existants ; aucune DLL, adresse
  d'instrument, MAC ou numéro de série publié.
- Quatre tâches : Mesurer, Contrôler, Profiler, Bibliothèque. Graphisme et
  vocabulaire propres ; aucun contenu propriétaire ou nuancier commercial fourni.

## Critères de fin

- [ ] Première livraison démontrée sur Epson pigmentaire / semigloss : profil RVB
  calculé puis vérifié avec rapport pour chacun des deux parcours instrument.
- [ ] Import FD-S2w exercé localement, avec provenance et association des plages vérifiées.
- [ ] Reprise, moyennage, données manquantes et refus de mesures incohérentes couverts.
- [ ] Premier flux réalisable sans #31, #32, #36–#39 ni #42 ; #40 ne dépend plus
  de #38 et #41 ne dépend plus du contrôle complet #37.
- [ ] À terme : contrôle d'un tirage, historique, rapports et étiquettes avec
  références et tolérances versionnées.

Les tickets #3 à #10 couvrent déjà instrument, étalonnage, bibliothèque, mesure
ponctuelle, couleur de référence, CGATS et spectres. Le contrôle d'éclairage reste
différé jusqu'à confirmation des capacités sûres. Modèle Epson exact, dimensions,
nombres de plages, réglages de profilage et seuils seront documentés lors des essais.
