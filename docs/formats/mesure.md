# Format des mesures (`myiro-libre/mesure/1`)

**En clair** : une mesure, c'est les valeurs lues par l'instrument sur une ou plusieurs plages, plus sa **provenance** (quel instrument, quelle chaîne logicielle, quand, comment). Le pont l'envoie à l'application dans ce format, et la bibliothèque la conserve dans ce même format. Le numéro de format, écrit en tête, permet de relire une mesure après une évolution du logiciel.

Le code est dans la crate `crates/pont-protocole` (fichier `src/mesure.rs`), indépendante de Windows, de la DLL et de l'application (ADR 0004). Les lignes du protocole propres au FD-9 (connexion par adresse, version, détection) sont décrites dans [protocole.md](protocole.md).

## Une mesure ne peut pas être mal formée

Chaque morceau d'une mesure est vérifié **quand on la fabrique comme quand on la relit** : il n'existe pas de chemin qui saute la vérification. Une mesure refusée l'est avec une phrase qui dit pourquoi.

| Morceau | Ce qui est exigé |
|---|---|
| spectre | au moins une valeur, toutes des nombres finis. Une réflectance **supérieure à 1** (papier azuré, fluorescence) est acceptée : c'est une mesure, pas une erreur |
| données brutes | des nombres finis |
| Lab | exactement trois nombres finis |
| plage | M0, M1 et M2 de même longueur |
| mesure | au moins une plage, une seule en mesure ponctuelle, toutes les plages de mêmes dimensions |
| MYIRO-1 | 36 valeurs par spectre (380 à 730 nm par 10 nm) et 152 données brutes par plage. Ces chiffres ne sont pas imposés aux autres instruments |
| provenance | modèle, version du pont, architecture et libellé du calcul non vides ; dates au format `2026-10-07T15:04:05+02:00` (fuseau obligatoire) ; empreinte de la DLL en 64 chiffres hexadécimaux minuscules |

Un champ que le format ne connaît pas est **refusé** : on ne devine pas ce qu'il voulait dire. Un nouveau champ demande un nouveau numéro de format.

## Exemple

```json
{
  "format": "myiro-libre/mesure/1",
  "plages": [
    {
      "m0": ["36 nombres"], "m1": ["36 nombres"], "m2": ["36 nombres"],
      "brutes": ["152 nombres"],
      "lab_m0": [50.1, 1.2, -2.3], "lab_m1": [50.4, 1.9, -6.0], "lab_m2": [50.0, 1.1, -1.8]
    }
  ],
  "provenance": {
    "instrument": { "modele": "MYIRO-1", "numero_serie": 12345678, "micrologiciel": "1.02.0005", "code_produit": "9C1D" },
    "version_sdk": [1, 0, 1],
    "empreinte_dll": { "statut": "confirmee", "valeur": "ab…ab (64 caractères)" },
    "version_pont": "0.1.0",
    "architecture": "x86_64",
    "horodatage": "2026-10-07T15:04:05+02:00",
    "etalonnage": { "statut": "confirmee", "valeur": "2026-10-07T15:00:00+02:00" },
    "geometrie": { "lecture": "ponctuelle" },
    "calcul": {
      "libelle": "spectres : Illuminant 0/1/2 = M0/M1/M2, 380-730 nm par 10 nm ; Lab : D50, 2°",
      "demande": { "statut": "confirmee", "valeur": {
        "conditions_spectres": [
          { "statut": "confirmee", "valeur": "M0" },
          { "statut": "confirmee", "valeur": "M1" },
          { "statut": "confirmee", "valeur": "M2" }
        ],
        "longueurs_onde": { "statut": "confirmee", "valeur": { "debut_nm": 380, "pas_nm": 10 } },
        "illuminant_lab": { "statut": "confirmee", "valeur": "D50" },
        "observateur_lab": { "statut": "supposee", "valeur": "2_degres" }
      } },
      "observe": { "statut": "inconnue" }
    }
  }
}
```

Dans le protocole, la même mesure arrive dans une réponse `{"rep": "mesure", "mesure": { … }, "remise_au_repos": { … }}`.

## Remise au repos (réponse du protocole seulement)

`remise_au_repos` dit si l'instrument est revenu au repos après la lecture (ticket #24). Ce n'est **pas** une donnée de la mesure conservée : il est à côté de `mesure`, dans la réponse du pont, et ne change pas le format `myiro-libre/mesure/1`. Il est qualifié comme les autres données :

- `{"statut": "confirmee", "valeur": {"etat": "au_repos"}}` : le pont a reçu la preuve du repos ;
- `{"etat": "repos_non_signale"}`, `{"etat": "arret_refuse", "code": -9987}` ou `{"etat": "liaison_perdue"}` (toujours sous `statut` et `valeur`) : la mesure reste valable, mais la suivante sera refusée tant que le repos n'est pas prouvé. Le `code` est celui de la DLL, brut ; -9987 (instrument muet dans le délai) est un exemple, confirmé par la fiche [`FDX_StopMeasurement`](../abi/FDX_StopMeasurement.md) ;
- `{"statut": "inconnue"}` : réponse écrite par un pont antérieur. Une réponse `mesure` sans ce champ, ou une ligne du format initial, se relit ainsi : **inconnu, jamais « au repos »**.

Une valeur nue (`{"etat": "au_repos"}` sans `statut`), un `etat` inconnu ou un champ en trop sont refusés.

Hors de la réponse `mesure`, la même valeur, sans `statut`, accompagne la réponse `fermeture_incertaine` et les erreurs `repos_incertain` et `deconnexion_echouee`. Elle peut alors valoir aussi `{"etat": "repos_suppose"}` : le désarmement a été refusé (-9986) sans événement alors que rien n'avait été armé. Ce refus est constaté au repos ; en déduire le repos reste une supposition, qui permet d'armer mais ne confirme pas une fermeture. Après un armement, ce cas ne se présente pas.

**Compatibilité.** Depuis le ticket #25, un lecteur refuse les champs et les valeurs qu'il ne connaît pas. Un lecteur antérieur au ticket #24 rejettera donc une réponse `mesure` qui porte `remise_au_repos`, la réponse `fermeture_incertaine` et les erreurs `repos_incertain`, `deconnexion_echouee` et `session_fermee` : l'application et le pont doivent être mis à jour ensemble. Autre changement visible : `fermer` juste après `connecter`, sans mesure, ne répond plus `ferme` mais `fermeture_incertaine` avec `repos_suppose`.

## Date d'étalonnage (réponse du protocole)

Un étalonnage réussi répond `{"rep": "etalonne", "date": "2026-10-07T09:30:00+02:00"}` (valeur fictive). La date est celle de l'horloge du pont, à la seconde, avec fuseau, au même format que `horodatage`. C'est exactement celle que porte ensuite `etalonnage` dans la provenance des mesures de cette connexion : l'application la garde telle quelle et ne la recalcule jamais (ticket #4).

Une réponse `etalonne` sans `date`, avec une date sans fuseau ou avec un champ en trop est refusée. **Compatibilité** : un pont antérieur répondait `{"rep": "etalonne"}` ; l'application actuelle le refuse, et un lecteur antérieur refuse le champ `date`. Application et pont se mettent à jour ensemble.

## Confirmé, supposé, inconnu

Une donnée dont on n'est pas sûr est **qualifiée** : `{"statut": "confirmee", "valeur": …}`, `{"statut": "supposee", "valeur": …}` ou `{"statut": "inconnue"}`. Une donnée inconnue n'a jamais de valeur, et une valeur nue à la place d'une donnée qualifiée est refusée.

- **Confirmé** : établi par une fiche `docs/abi/` confirmée, ou observé par le pont lui-même (empreinte qu'il a calculée, heure de l'étalonnage réussi).
- **Supposé** : plausible, pas encore vérifié. Aujourd'hui : l'observateur 2° (code 0), d'après la fiche [`FDX_GetMeasureData`](../abi/FDX_GetMeasureData.md).
- **Inconnu** : personne ne l'a fourni.

Les autres champs de la provenance (instrument, version du SDK, version et architecture du pont, heure de la mesure) sont toujours observés par le pont : ils ne sont pas qualifiés.

## Géométrie

`geometrie` dit comment l'instrument a lu : `{"lecture": "ponctuelle"}`, `{"lecture": "bande", "sens": 0}` ou `{"lecture": "feuille"}`. Le `sens` d'une bande est la valeur brute rendue par la DLL (2 : plages dans l'ordre inverse).

## Conditions demandées et conditions observées

`calcul` sépare deux choses :

- `demande` : ce que le pont a **demandé** à la DLL (conditions M0, M1, M2 des trois spectres, longueurs d'onde, illuminant et observateur des Lab) ;
- `observe` : ce qui a été **relu sur l'instrument**. Le pont actuel ne relit rien : `observe` vaut toujours `{"statut": "inconnue"}`. Il ne passera à autre chose qu'avec un appel vérifié qui rend ces conditions.

`libelle` garde la description en clair écrite par le pont.

## Lecture du format initial

Avant ce format, le pont 0.1.0 écrivait ses mesures sans numéro de format :

```json
{"rep": "mesure", "plages": [ … ], "sens": 0,
 "provenance": { …, "empreinte_dll": "…", "etalonnage": "…", "geometrie": "ponctuelle", "calcul": "texte libre" }}
```

Ces lignes se relisent toujours (`lire_mesure`, `lire_reponse`), sans toucher aux fichiers d'origine : la reprise se fait en mémoire, rien n'est réécrit ni effacé dans `Archivage/donnees/`. Seuls les faits démontrables sont repris :

| Champ du format initial | Devient |
|---|---|
| plages, instrument, version du SDK, version et architecture du pont, heure de la mesure | repris tels quels, puis vérifiés comme ci-dessus |
| `empreinte_dll` présente | `confirmee` (le pont l'avait calculée lui-même) ; absente : `inconnue` |
| `etalonnage` présent | `confirmee` ; `null` : `inconnue` |
| `geometrie` `"ponctuelle"` | `{"lecture": "ponctuelle"}` ; le `sens` 0 est ignoré, le pont 0.1.0 l'écrivait sans l'avoir lu |
| `geometrie` `"bande"` | `{"lecture": "bande", "sens": …}` avec le `sens` de la ligne |
| `calcul` (texte libre) | gardé comme `libelle` ; `demande` et `observe` sont `inconnue`, car rien ne peut être déduit d'un texte libre |

Une ligne irrécupérable est refusée avec son explication : géométrie inconnue, champ manquant ou en trop, longueur erronée, nombre non fini, date sans fuseau. Les CSV des tests sur instrument ne sont pas des mesures au sens de ce format ; ils restent lus par le jeu de validation ([`jeu-validation.md`](../pilote-libre/jeu-validation.md)).

Une mesure reprise du format initial peut ensuite être écrite au format courant ; elle garde ses champs `inconnue`.

## Version non prise en charge

Une mesure dont le `format` n'est pas `myiro-libre/mesure/1` est refusée avant tout examen de son contenu, avec un message qui cite le format trouvé (« format de mesure « myiro-libre/mesure/2 » non pris en charge »). Une mesure sans `format` (hors format initial) est refusée aussi.
