# FD9_StopMeasurement

**En clair** : demande à la DLL d'**interrompre** le travail en cours sur une feuille (attente de la feuille, image, reconnaissance, mesure). Elle ne fait que poser la demande et rend la main tout de suite ; l'arrêt réel est annoncé ensuite par l'événement 10 (`WAIT_CANCEL_COMPLETION`). Son argument dit si l'instrument doit enchaîner sur la feuille suivante.

## Signature

```c
int32_t __stdcall FD9_StopMeasurement(const bool *feuille_suivante);  /* 1 octet ; x86 : ret 4 */
```

## Ce qui est confirmé

- L'export (`fd9-x86` `0x100b1250`) passe le pointeur à la fonction de travail `0x1008b560`, sans test du pointeur nul.
- **Hors connexion** : code interne 8065, public **1102** (`0x1008b5b2..0x1008b5ba`).
- **Admise seulement dans les états 1 à 8** du SDK (`WAIT_AUTHORITY` à `RUN_MEASURE`, `0x1008b5c1..0x1008b5ee`). Dans tout autre état, **y compris 9 (`WAIT_MEASURE_COMPLETION`)**, 10 et les états d'erreur : code interne 8066, public **1104**, sans effet.
- Admise, elle **pose un drapeau d'arrêt** (`0x10221dc8` ← 1, `0x1008b5f5`) et **garde la valeur du booléen** (`0x10222388`, `0x1008b601`), puis rend 0. Aucune commande n'est envoyée dans cette fonction : l'arrêt est fait par le fil de travail de la DLL.
- Le booléen gardé est relu par [FD9_ExitMeasurement](FD9_ExitMeasurement.md) quand l'état est 10 (`0x1008c42e..0x1008c45f`).

## Quand MYIROtools l'appelle

Lu dans le gestionnaire d'événements de MYIROtools, le logiciel du fabricant qui imprime et mesure la RGB-2520 (`fd9-feuille.md` § 3) :

- **en cours de route**, quand un de ses appels échoue : `FD9_StopMeasurement(&faux)` ;
- **à l'événement 9** (mesure finie) : attente d'une seconde, puis `FD9_StopMeasurement(&vrai)`, puis `FD9_ExitMeasurement(&0, NULL)`.

D'après les contrôles ci-dessus, l'appel de l'événement 9 est refusé (1104) si l'état est encore 9 à ce moment ; la suite de MYIROtools ne dépend pas de son retour.

Quand l'opérateur **annule** (`0x1403ec140`, journal « setCancelMeasurement ») : MYIROtools lit l'état par `FD9_GetStatus` (mot +0x10) et n'appelle `FD9_StopMeasurement` **que dans les états 2 à 8**. Dans les états 0 et 1 (attente de la feuille, rien d'engagé) et à partir de 9, il n'appelle rien et rend la main (confirmé).

Sa **déconnexion** (`0x1403eb7f0`) : `FD9_ExitMeasurement(&0, NULL)` sans lire le retour, puis `FD9_Disconnect`, puis `FD9_GetStatus` toutes les 100 ms jusqu'à l'état 1 (code 0) ou au code 1102 (plus de session), **sans limite de temps** (confirmé).

## Ce qu'a montré l'instrument

- **Confirmé par E2 (28 octobre 2026)** : pas d'événement 10 en 30 s après `FD9_StopMeasurement(&faux)` rendu 0 depuis l'état 1 (attente de la feuille, aucune feuille glissée). Le `FD9_Disconnect` qui a suivi n'a pas rendu la main en près de 5 minutes ; le processus a été arrêté de force.
- Supposé : le drapeau d'arrêt n'est lu que par les fils de travail de la DLL (feuille, mesure) ; à l'état 1, aucun ne tourne, donc rien ne produit l'événement 10, et l'arrêt de ces fils dans `FD9_Disconnect` (`0x1008af80`) reste bloqué. La cause exacte du blocage n'est pas établie.
- Aucun délai interne d'attente de la feuille n'a été trouvé dans la DLL (seule chaîne de délai : celle du fil de vie de la liaison) : inconnu, rien ne dit que la DLL abandonne seule.

## Propriété de la mémoire

- Le booléen appartient à l'appelant ; la DLL en lit un octet pendant l'appel et garde sa **valeur**, pas son adresse (confirmé).

## Ce qui est supposé

- `vrai` = l'instrument attend une nouvelle feuille après l'arrêt ; `faux` = il rend la feuille et s'arrête (nom d'origine `bNextFeed`).
- Après un arrêt admis dans les états 2 à 8, l'état passe à 10 et l'événement 10 est envoyé au rappel (faux depuis l'état 1, voir E2).
- À l'événement 9, l'attente d'une seconde de MYIROtools ne change pas l'état ; son `FD9_StopMeasurement(&vrai)` est alors sans effet.

## Pour le pont

- Ne l'appeler que pour **annuler** pendant les états 1 à 8 (délai de la feuille dépassé, autre mire, mire non reconnue, annulation de l'opérateur), avec `faux`.
- Après un arrêt admis, **attendre l'événement 10** avant [FD9_ExitMeasurement](FD9_ExitMeasurement.md) : appelée plus tôt, celle-ci est refusée. Sans événement 10 dans le délai, rendre `Delai` sans inventer d'état.
- À l'événement 9, ne pas compter sur `FD9_StopMeasurement` : un 1104 à cet endroit est journalisé, pas traité comme une panne.
- Un arrêt peut croiser la fin de la mesure (supposé, rien ne l'exclut dans le code lu) :
  - si l'événement 9 arrive à la place de l'événement 10, la feuille est finie : le pont la clôt par `FD9_ExitMeasurement`, admis à l'état 9, et rend la raison de l'arrêt ;
  - si l'arrêt est refusé (1104), le pont laisse au rappel jusqu'à 2 s pour annoncer l'état où la DLL est déjà passée. À 9 ou 10, il clôt la feuille ; sur un état d'erreur, il traite l'incident ; sinon, il rend le refus.

## À vérifier sur l'instrument

- La feuille est-elle éjectée ou retenue après `FD9_StopMeasurement(&faux)` (essais E2 et E3) ?
- ~~L'événement 10 arrive-t-il après un arrêt en attente de feuille (état 1) ?~~ Non (E2, voir plus haut).
- Le code rendu par `FD9_StopMeasurement(&vrai)` à l'événement 9 (essai E4).

## Preuves (locales)

- `fd9-x86/exports/FD9_StopMeasurement.asm.txt` (`ret 4`) ; `fd9-x64/exports/FD9_StopMeasurement.asm.txt`.
- Fonction de travail : `desassemble.py fd9-x86 --address 0x1008b560`.
- Codes publics : table de regroupement `0x10108ca0` (8065 → 1102, 8066 → 1104).
- Séquence de MYIROtools : `retroanalyse/logiciels/fd9-feuille.md` § 3 ; FD-S2w : `fd-s2w.md` § 5.
