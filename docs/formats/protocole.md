# Protocole des ponts : lignes propres au FD-9 et mesure sans le bouton

L'application parle à chaque pont par une ligne JSON par message, dans chaque sens (crate `pont-protocole`, ADR 0005). Les mesures et la remise au repos sont décrites dans [mesure.md](mesure.md). Cette page décrit les trois lignes ajoutées pour le FD-9 (ticket #13) et le déclenchement de la mesure ponctuelle du MYIRO-1 (ticket #51). Les règles sont les mêmes : un champ ou une valeur inconnus sont refusés, et une donnée incertaine est **qualifiée** (`confirmee`, `supposee` ou `inconnue`, voir [mesure.md](mesure.md#confirmé-supposé-inconnu)).

Les adresses et identifiants ci-dessous sont fictifs.

## Se connecter à une adresse : `connecter_adresse`

Requête de l'application. C'est le paramètre de connexion propre au FD-9 : son adresse IP ou son nom sur le réseau, sans passer par la détection.

```json
{"cmd": "connecter_adresse", "adresse": "192.0.2.40"}
```

- `adresse` : 1 à 23 caractères ASCII visibles (ni espace ni accent). La DLL n'en recopie que 24 octets sans zéro final (fiche [FD9_Connect](../abi/FD9_Connect.md)) : une adresse plus longue est refusée à la lecture (`requete_invalide`). Le port, 49152, est fixé dans la DLL.
- Le pont MYIRO-1 répond `requete_invalide` sans appeler la DLL. Le pont FD-9 n'a pas encore de palier Connexion : il répond `palier_non_autorise` (plafond `detection`).

## Version d'une DLL sans fonction de version : `version_dll`

Réponse du pont FD-9 à `version`. FD9SDK n'a pas de fonction publique de version (fiche [FD9_GetLastError](../abi/FD9_GetLastError.md)) : le pont lit la version écrite dans la ressource du fichier et calcule son empreinte SHA-256, puis appelle `FD9_GetLastError` pour prouver que la DLL répond.

```json
{"rep": "version_dll",
 "version_fichier": {"statut": "confirmee", "valeur": [1, 3, 2, 3]},
 "empreinte": {"statut": "confirmee", "valeur": "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"}}
```

- `version_fichier` : majeur, mineur, révision, build. Illisible : `{"statut": "inconnue"}`.
- `empreinte` : 64 chiffres hexadécimaux minuscules. Illisible : `{"statut": "inconnue"}`.
- Les deux champs sont obligatoires : une donnée absente est refusée, jamais lue comme inconnue.

## FD-9 détectés : `instruments_fd9`

Réponse du pont FD-9 à `detecter`. Une liste vide veut dire « aucun FD-9 visible », pas une erreur.

```json
{"rep": "instruments_fd9", "liste": [
  {"liaison": {"statut": "confirmee", "valeur": "reseau"},
   "adresse": "192.0.2.40",
   "identifiant": {"statut": "supposee", "valeur": "12345678"}}
]}
```

- `liaison` : `reseau` ou `usb`, confirmés par la fiche [FD9_GetDeviceList](../abi/FD9_GetDeviceList.md). Un autre code de la DLL donne `{"statut": "inconnue"}` (le code brut reste au journal du pont) ; un texte libre est refusé.
- `adresse` : adresse IP en réseau, `COMn` en USB.
- `identifiant` : les 8 caractères rendus par la DLL, tels quels. Qu'ils désignent l'instrument (son n° de série) est **supposé** ; vides, ils sont `{"statut": "inconnue"}`.

## Mesure sans le bouton : `declenchement` (MYIRO-1, ticket #51)

Champ facultatif de la requête `mesurer_ponctuelle`. Il dit qui fait partir la mesure une fois l'instrument armé.

```json
{"cmd": "mesurer_ponctuelle", "declenchement": "automatique"}
```

- `manuel` (valeur par défaut) : l'instrument attend l'appui sur son bouton, comme avant. L'application écrit alors la ligne d'avant, sans le champ : `{"cmd": "mesurer_ponctuelle"}`.
- `automatique` : le pont attend que l'instrument soit armé (événement 1, 5 s au plus), puis appelle `FDX_StartMeasurement` (fiche [FDX_StartMeasurement](../abi/FDX_StartMeasurement.md)) et attend la fin de la mesure (30 s au plus). Il ne l'appelle jamais sans cet événement. Si l'opérateur appuie quand même sur le bouton avant, la mesure faite est gardée ; après un refus -9986, le pont attend encore 2 s qu'une mesure partie au bouton se signale avant de désarmer (supposé : un appui juste avant le déclenchement le ferait refuser).
- Toute autre valeur est refusée à la lecture (`requete_invalide`).

Si la DLL ou l'instrument refuse le déclenchement, le pont désarme et répond une erreur avec le code brut :

```json
{"rep": "erreur", "erreur": {"type": "declenchement_refuse", "code": -9986}}
```

-9986 : l'instrument n'attendait pas de mesure ; -9793 à -9789 : refus rapporté par l'instrument (supposé). L'étalonnage reste valable : la mesure au bouton reste possible.

## Annuler une mesure en attente : `annuler` (ticket #26)

```json
{"cmd": "annuler"}
```

Le pont lit ses requêtes pendant qu'une mesure attend : `annuler` est vue tout de suite, sans attendre la fin de la mesure. Les appels à la DLL restent faits un par un, et **chaque requête reçoit une seule réponse, dans l'ordre d'arrivée** : d'abord celle de la mesure, puis celle de `annuler`.

- `annuler` vise la dernière requête qui la précède (hors autre `annuler`), jamais la suivante.
- Elle n'a d'effet que tant que la mesure attend l'appui sur le bouton (ou, en automatique, l'attente de mesure avant le déclenchement). Le pont désarme alors par la politique habituelle et la mesure répond :

  ```json
  {"rep": "erreur", "erreur": {"type": "mesure_annulee", "remise_au_repos": {"etat": "au_repos"}}}
  ```

  Rien n'a été lu. Un repos non prouvé bloque la mesure suivante (`repos_incertain`), comme après toute mesure.
- Une mesure déjà partie (événement 2) ou terminée va à son terme : sa réponse est la mesure, avec sa provenance. Aucun abandon n'est annoncé sans être fait.
- Puis `annuler` reçoit `{"rep": "annulation", "effet": "appliquee"}` si la mesure a été interrompue, `{"rep": "annulation", "effet": "sans_effet"}` sinon (rien à interrompre).

La fin de l'entrée (application fermée ou perdue) interrompt de même une mesure qui attend l'opérateur, puis le pont se ferme. Le pont FD-9, qui ne mesure pas, répond toujours `sans_effet`.

Côté application, après l'envoi de `annuler`, chaque réponse est attendue au plus le délai ordinaire (30 s) ; au-delà, le pont est arrêté de force et l'instrument est dit dans un état incertain.

## Compatibilité

La commande `annuler`, la réponse `annulation` et l'erreur `mesure_annulee` (ticket #26) sont refusées par un lecteur antérieur : l'application et les ponts se mettent à jour ensemble. Aucune ligne existante ne change.

Le champ `declenchement` est absent des lignes manuelles : un pont antérieur au ticket #51 les lit comme avant. Mais il refuse une requête automatique (champ inconnu), et une application antérieure refuse l'erreur `declenchement_refuse` : l'application et les ponts se mettent à jour ensemble.

Pour le FD-9, aucune ligne existante ne change : le MYIRO-1 écrit et lit toujours `version` et `instruments` comme avant. Mais un lecteur antérieur au ticket #13, qui refuse depuis le ticket #25 les commandes et réponses inconnues, rejettera `connecter_adresse`, `version_dll` et `instruments_fd9` : l'application et les ponts se mettent à jour ensemble.
