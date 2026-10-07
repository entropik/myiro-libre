# FD9_Disconnect

**En clair** : ferme la session avec le FD-9 et libère la prise de main, pour que FD-S2w ou un autre logiciel puisse s'en servir. La DLL refuse si une mesure est en cours.

## Signature

```c
int32_t __stdcall FD9_Disconnect(void);   /* x86 : ret (aucun argument) */
```

## Ce qui est confirmé

- Aucun argument.
- **Sans session ouverte** : code interne 8054, public **1004** (`fd9-x86` `0x1008af80..0x1008af89`).
- **Pendant une mesure** : si l'état du SDK n'est ni 0 ni 1, code interne 8064, public **1104**, et la session reste ouverte (`0x1008af8f..0x1008af9d`).
- Sinon, la DLL arrête ses objets de communication et d'événements, ferme le transport, remet le drapeau de session à zéro et renvoie 0 (`0x1008afa3..0x1008b004`).

## Propriété de la mémoire

- Aucun argument, aucun pointeur rendu (confirmé).
- La DLL libère elle-même ses objets de session : objet d'événements arrêté puis détruit (`0x1008afa3..0x1008afc2`) (confirmé) ; transport fermé par l'appel suivant (`0x1008aff5`, rôle supposé). L'appelant n'a rien à libérer.
- L'adresse du rappel d'événements reste enregistrée (aucune écriture dans sa variable) : le pont la désinscrit lui-même ensuite (confirmé, voir [FD9_RegisterDeviceEventHandler](FD9_RegisterDeviceEventHandler.md)).

## Ce qui est supposé

- Les états 0 et 1 sont les deux états d'attente de prise de main de `FD9_eSDKStatus` ; tout autre état veut dire qu'un travail ou une mesure est engagé.
- Le rappel d'événements n'est plus appelé après le retour (l'objet qui l'appelle est détruit, mais l'attente d'un appel en cours n'a pas été relevée).

## Pour le pont

- Appeler à la fermeture de l'adapter, et après un échec en cours de session ; un code 1004 à la fermeture n'est pas une anomalie.
- Sur 1104, ne pas insister : arrêter d'abord la mesure (fonctions hors du périmètre de ces fiches).
- Désinscrire ensuite le rappel (`FD9_RegisterDeviceEventHandler` avec un pointeur nul).

## À vérifier sur l'instrument

- Après la déconnexion, FD-S2w doit pouvoir reprendre la main sans redémarrer l'instrument.

## Preuves (locales)

- `fd9-x86/exports/FD9_Disconnect.asm.txt` (`ret` sans argument) et `fd9-x64/exports/FD9_Disconnect.asm.txt`.
- Session : `desassemble.py fd9-x86 --address 0x1008af80`.
