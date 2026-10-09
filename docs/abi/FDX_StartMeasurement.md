# FDX_StartMeasurement

**En clair** : c'est l'équivalent logiciel de l'appui sur le bouton du MYIRO-1. Une fois la mesure armée par `FDX_SetMeasureCondition`, cette fonction demande à l'instrument de mesurer tout de suite, sans qu'on touche à son bouton. La DLL ne l'interdit pas pour une mesure ponctuelle en réflexion, mais aucun logiciel du fabricant ne s'en sert ainsi, et elle **n'est pas dans la liste blanche** du pont : son entrée attend l'accord du mainteneur (ticket #51).

## Signature

```c
int32_t __stdcall FDX_StartMeasurement(void);   /* x86 : ret sans opérande, aucun argument */
```

Aucun argument : le type de mesure est celui qui a été armé par `FDX_SetMeasureCondition` (confirmé : `ret` simple en x86, signature sans paramètre dans les SDK Mac).

## Ce qui est confirmé (lecture statique)

- **Il faut d'abord armer.** La fonction attend l'état 2 (« attente de mesure », celui de l'événement 1) : dix essais espacés de 5 ms, puis refus **-9986** (« can not use. now status ») si l'instrument n'y est pas. Elle ne peut donc rien déclencher au repos.
- **Elle ne regarde pas le type de mesure.** Aucun contrôle de `MeasureType` sur son chemin, ni dans la fonction exportée, ni dans la routine interne qu'elle appelle : rien dans la DLL ne la réserve à la lumière ambiante ou à l'écran.
- **Elle envoie la commande de démarrage.** La routine interne (`CInstrument::StartMeasurement`, journal « start measurement. ») envoie la commande `0x11` avec le paramètre **1** (démarrer). `FDX_StopMeasurement` envoie la même commande avec le paramètre **2** (arrêter). Ce sont les deux seuls endroits de la DLL 1.0.1 x86 qui fabriquent cette commande.
- **Puis elle attend que la mesure parte** : elle rend la main quand l'instrument passe à l'état 3 (« mesure en cours », celui de l'événement 2), ou refuse -9986 s'il retombe au repos ou se déconnecte. Le retour de la fonction ne veut pas dire que les données sont prêtes : il faut toujours attendre l'événement 3.
- **Les refus de l'instrument** pendant ce démarrage (états d'erreur rapportés par la commande `0x11`) deviennent les codes -9793 à -9789, comme pour `FDX_StopMeasurement`.
- **Aucune écriture permanente** sur ce chemin : une seule commande, qui démarre une acquisition.
- **Même structure dans la DLL 1.0.1 x64** (celle d'Ergosoft, utilisée par le pont 64 bits) : attente de l'état 2, appel de la routine de démarrage, attente de l'état 3.

## Qui l'appelle (confirmé)

| Logiciel | Usage | Type armé |
|---|---|---|
| MYIRO tools (Mac et Windows) | mesure de l'illuminant de l'utilisateur seulement | 2 (lumière ambiante) |
| EIZO ColorNavigator 7 | mesure d'écran (spectre et XYZ) | 3 (écran) |
| MY-CT1 | ne l'importe pas | — |

Pour la mesure en réflexion, ponctuelle ou en bande, MYIRO tools arme puis **laisse le bouton de l'instrument** déclencher. Aucun appelant connu ne déclenche une mesure en réflexion par logiciel.

## Ce qui est supposé

- **La DLL traite la mesure ponctuelle comme la lumière ambiante.** Dans le SDK Mac 1.0.5, les types 0 (ponctuelle) et 2 (lumière ambiante) passent par la même routine de mesure (`otherScanMeasurement`), et MYIRO tools déclenche le type 2 par logiciel. Une mesure ponctuelle déclenchée ainsi suivrait donc, côté DLL, le même chemin qu'un appui sur le bouton : événements 2, puis 3 (supposé fort).
- **L'instrument l'accepte en réflexion.** Rien dans la DLL ne permet de savoir si le micrologiciel du MYIRO-1 accepte la commande de démarrage quand une mesure en réflexion est armée. S'il la refuse, l'appel devrait rendre un code de -9793 à -9789 (supposé). **À vérifier sur l'instrument.**
- **La bande ne s'y prête pas.** En bande, l'opérateur fait glisser l'instrument bouton enfoncé ; un déclenchement logiciel n'a pas de sens (supposé : seul le type 0 est concerné ici).
- **Après la mesure**, l'instrument se réarme seul (événement 1), comme après un appui sur le bouton : le désarmement du pont (ADR 0005, ticket #24) reste le même (supposé).

## Ce qui n'existe pas

- **Aucun réglage de `FDX_SetMeasureCondition`** ne fait partir la mesure seule : son champ `option` est le nombre de plages d'une bande, et l'armement s'arrête à l'état 2 (fiche [FDX_SetMeasureCondition](FDX_SetMeasureCondition.md)).
- Aucun autre export ne démarre une mesure : `FDX_StartMeasurement` est le seul chemin vers la commande de démarrage hors de la mesure d'écran.
- À rapprocher de la fiche `FDX_SetMeasureCondition`, qui indique « commande `0x02` puis démarrage (`0x11`) » : dans la 1.0.1 x86, l'armement envoie la commande `0x02` ; la commande `0x11` de démarrage n'est fabriquée que par la routine décrite ici, appelée par cet export et par la mesure d'écran.

## Classement

Export sans argument, ni `FDX_Set*` ni `FDX_JIG_*`, qui ne fait que démarrer une acquisition déjà armée, par la même commande que `FDX_StopMeasurement` (déjà autorisé) avec un autre paramètre. Risque matériel jugé faible (supposé). **Hors liste blanche** : son entrée dans `crates/fdx-sys` demande l'accord du mainteneur, puis un essai sur l'instrument réel au palier Mesure ponctuelle.

## Pour le pont (si l'accord est donné)

- Seulement avec le type 0 armé, après l'événement 1 ; jamais au repos, jamais en bande.
- Après l'appel : attendre les événements 2 et 3 comme aujourd'hui, avec un délai court (la mesure part tout de suite, pas d'attente d'un appui) ; un code de -9793 à -9789 veut dire que l'instrument refuse le déclenchement logiciel : revenir au mode manuel, sans le présenter comme une panne.
- Désarmement inchangé.

## Reste à vérifier sur l'instrument

1. Armer une mesure ponctuelle, attendre l'événement 1, appeler `FDX_StartMeasurement` sans toucher au bouton : code rendu, événements reçus (2 puis 3 attendus), voyant.
2. Comparer la mesure à une mesure du même blanc faite au bouton.
3. Vérifier que le réarmement automatique et le désarmement suivent comme après un appui.

## Preuves (locales)

- DLL x86 1.0.1 : `fdx-x86/exports/FDX_StartMeasurement.asm.txt` (export à `0x10036030` ; attente de l'état 2 à `0x10036080..0x100360fb`, refus -9986 à `0x10036139` ; appel de la routine de démarrage à `0x1003619a` ; attente de l'état 3 à `0x100361a3..0x100361dd`).
- Routine interne `CInstrument::StartMeasurement` à `0x10025f70` (paramètre 1 posé à `0x10025fb4`, envoi à `0x10026002`) ; `CInstrument::StopMeasurement` à `0x100260b0` (paramètre 2 posé à `0x100260f8`). Les deux seules références à l'objet de commande `0x11` sont à `0x10025fa7` et `0x100260e9`. Appelants de `0x10025f70` : `0x1003619a` (cet export) et `0x10030b95` (`CMeasurement::radianceMeasurement`, mesure d'écran).
- Armement : `CInstrument::SetMeasureCondition` à `0x10025e60` (commande `0x02`), appelé depuis `0x1001ccc5`.
- DLL x64 1.0.1 : `CFDX::StartMeasurement` à `0x1800234c0` (état 2 à `0x180023532` et `0x1800235fd`, démarrage à `0x180023680`, état 3 à `0x18002369a`) ; routine de démarrage à `0x18002bbd0` (paramètre 1 à `0x18002bc18`), appelée aussi depuis `0x180037b6e`.
- Appelants : `retroanalyse/logiciels/myiro-tools.md` § 6 et § 13 (illuminant, `0x1004980b3`) ; EIZO `preuves/eizo-x64/mesure-spectrale.asm.txt` (`0x18004245c`) et `mesure-xyz.asm.txt` (`0x180042fed`) ; imports de MY-CT1 dans `myct1/metadata.json`.
