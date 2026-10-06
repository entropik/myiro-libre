# myiro-libre

Outil libre de mesure et de profilage ICC pour les spectrophotomètres Konica Minolta **MYIRO-1** et **FD-9**, dont le logiciel Myiro Tool n'est plus maintenu.

Objectif : permettre aux imprimeurs et utilisateurs de continuer à mesurer des chartes, des couleurs ponctuelles et des densités, et à calculer des profils ICC, avec une application autonome et open source écrite en Rust.

**État : conception.** Aucune application utilisable pour l'instant. Le plan de travail est dans [PLAN-ACTION.md](PLAN-ACTION.md).

## Principes

- Projet libre, non commercial, sous licence [GPL-3.0](LICENSE).
- Aucun binaire, SDK ni manuel Konica Minolta n'est distribué dans ce dépôt : l'application utilise les DLL déjà installées chez l'utilisateur.
- Sécurité de l'instrument d'abord : les fonctions de maintenance usine des SDK ne sont jamais appelées.

## Contribuer

Les tickets sont suivis dans les GitHub Issues. La documentation et les échanges sont en français.
