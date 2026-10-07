# FDX_StopMeasurement

**En clair** : désarme une mesure armée par `FDX_SetMeasureCondition` et ramène l'instrument au repos. Sert avant d'étalonner, avant de réarmer, et à la fin d'une session.

## Signature

```c
int32_t __stdcall FDX_StopMeasurement(void);   /* x86 : ret sans opérande, aucun argument */
```

## Ce qui est confirmé

- Aucun argument (`ret` simple en x86, signature sans paramètre dans les SDK Mac).
- Refus -9986 si l'état ne s'y prête pas ; -9987 si l'instrument ne répond pas dans le délai.
- Commande envoyée : arrêt de mesure (`0x11`) ; les états d'erreur rapportés par l'instrument pendant le démarrage ou l'arrêt deviennent les codes -9793 à -9789.
- MYIRO tools l'appelle avant chaque armement et avant chaque étalonnage ; EIZO, après la lecture des données.

## Vérifié sur l'instrument

Le 7 octobre 2026 : refusé (-9986) au repos, et aussi **juste après une mesure** (état « mesure réussie »), jusqu'au réarmement automatique de l'instrument (événement 1). Accepté ensuite, suivi de l'événement 0 (retour au repos). Voir `FDX_SetMeasureCondition`.

## Pour le pont

- Appeler avant de réarmer, avant d'étalonner, et avant `FDX_Disconnect` si une mesure est armée.
- Un refus -9986 au repos n'est pas grave : rien n'était armé. Juste après une mesure, attendre l'événement suivant et réessayer.
- À la fermeture d'une session, toujours désarmer ainsi avant `FDX_Disconnect`.

## Preuves (locales)

- `fdx-x86/exports/FDX_StopMeasurement.asm.txt` (refus -9986 à `0x1003644c`, `ret` à `0x10036588`).
- `retroanalyse/logiciels/myiro-tools.md` § 9 et § 13.
