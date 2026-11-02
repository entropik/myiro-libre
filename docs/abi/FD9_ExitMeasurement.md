# FD9_ExitMeasurement

**En clair** : clôt le traitement d'une feuille, une fois qu'il est fini, annulé ou tombé en erreur. La DLL efface alors ce qu'elle gardait de cette feuille (QR, état de la mesure) et revient en attente. Le premier argument peut demander d'afficher une image de résultat sur l'écran de l'instrument : le pont ne s'en sert pas.

## Signature

```c
int32_t __stdcall FD9_ExitMeasurement(const uint32_t *afficher_resultat,             /* 0 ou 1 */
                                      const tFD9_DisplayImage *image_resultat);    /* NULL si 0 ; x86 : ret 8 */
```

## Ce qui est confirmé

- **Contrôles de l'export** (`fd9-x86` `0x100b15f0`) : `*afficher_resultat` au plus 1, et `image_resultat` non nul quand il vaut 1 ; sinon **1001** (`0x100b1619..0x100b1632`). Le premier pointeur est lu sans test du pointeur nul.
- **Hors connexion** : code interne 8009, public **1102** (`0x1008c2e2..0x1008c2ea`).
- **Admise seulement dans un état de fin** (`0x1008c2f4..0x1008c345`) : 9 (`WAIT_MEASURE_COMPLETION`), 10 (`WAIT_CANCEL_COMPLETION`), 101 à 103, 201 à 206, 301. Dans les états 1 à 8 : code interne 8050, public **1104**, et rien n'est effacé. Exception : à l'état 103 (communication), le nettoyage a lieu même en cas de refus (`0x1008c4ac`).
- **`afficher_resultat` = 1** : la DLL construit une image et l'envoie à l'instrument pour affichage (`0x1008c35a..0x1008c3c0`, « DisplayImage »). Avec 0, rien n'est envoyé pour l'affichage.
- **Nettoyage** (`0x1008c4b5..0x1008c583`) : arrêt de fils de travail, remise à zéro des drapeaux de la feuille et du drapeau d'arrêt de [FD9_StopMeasurement](FD9_StopMeasurement.md), **effacement du QR gardé** (1 172 octets à `0x10221e9c`), puis passage à un nouvel état interne.
- À l'état 10, la DLL relit le booléen gardé par `FD9_StopMeasurement` (`0x1008c42e`).

## Quand MYIROtools l'appelle

Lu dans le gestionnaire d'événements de MYIROtools (`fd9-feuille.md` § 3), toujours avec `(&0, NULL)` :

- à l'événement 9 (mesure finie), après une seconde d'attente et `FD9_StopMeasurement(&vrai)` ;
- à l'événement 10 (annulation finie) ;
- aux événements d'erreur 101 à 103, 202 à 206, 301 ; et à 201 après [FD9_GetPremeasurementError](FD9_GetPremeasurementError.md).

## Propriété de la mémoire

- Avec `(&0, NULL)`, la DLL ne lit qu'un nombre de l'appelant, pendant l'appel (confirmé).
- Avec une image, elle la recopie dans un tampon qu'elle alloue et libère elle-même avant le retour (`0x1008c35c..0x1008c3cb`) (confirmé) ; le pont n'utilise pas ce chemin.

## Ce qui est supposé

- Après le nettoyage, l'état revient à 1 (`WAIT_AUTHORITY`) : l'instrument attend une nouvelle feuille.
- Les données des plages ne sont plus lisibles par [FD9_GetData](FD9_GetData.md) après cet appel.

## Pour le pont

- Toujours `FD9_ExitMeasurement(&0, NULL)` : jamais d'image envoyée à l'instrument.
- L'appeler **seulement après avoir reçu** l'événement 9, 10 ou un événement d'erreur ; après un `FD9_StopMeasurement`, attendre d'abord l'événement 10.
- Lire toutes les plages **avant** cet appel.
- Un refus (1104) est rendu tel quel avec l'état reçu en dernier ; la session reste fermable par `FD9_Disconnect`.
- Après une attente de feuille finie sans feuille aux états 0 ou 1, ne pas l'appeler (voir [FD9_StopMeasurement](FD9_StopMeasurement.md)).
- **À la fermeture**, comme MYIROtools : l'appeler une fois avant [FD9_Disconnect](FD9_Disconnect.md), **sans tenir compte du retour** (journalisé). Sans feuille à clore, elle est refusée (1104) et n'a pas d'effet.

## À vérifier sur l'instrument

- L'état reçu juste après (événement 1 ?), et si une nouvelle feuille peut suivre sans reconnexion (essais E2 et E4).
- L'effet de `FD9_ExitMeasurement` sur la feuille encore engagée après une erreur.

## Preuves (locales)

- `fd9-x86/exports/FD9_ExitMeasurement.asm.txt` (contrôles, `ret 8`) ; `fd9-x64/exports/FD9_ExitMeasurement.asm.txt`.
- Fonction de travail : `desassemble.py fd9-x86 --address 0x1008c2b0`.
- Codes publics : table de regroupement `0x10108ca0` (8009 → 1102, 8050 → 1104).
- Signature : `retroanalyse/logiciels/fd-s2w.md` § 4.2 ; séquences : `fd-s2w.md` § 5, `fd9-feuille.md` § 3.
