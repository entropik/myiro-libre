# Protocole des ponts : lignes propres au FD-9

L'application parle à chaque pont par une ligne JSON par message, dans chaque sens (crate `pont-protocole`, ADR 0005). Les mesures et la remise au repos sont décrites dans [mesure.md](mesure.md). Cette page décrit les trois lignes ajoutées pour le FD-9 (ticket #13). Les règles sont les mêmes : un champ ou une valeur inconnus sont refusés, et une donnée incertaine est **qualifiée** (`confirmee`, `supposee` ou `inconnue`, voir [mesure.md](mesure.md#confirmé-supposé-inconnu)).

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

## Compatibilité

Aucune ligne existante ne change : le MYIRO-1 écrit et lit toujours `version` et `instruments` comme avant. Mais un lecteur antérieur au ticket #13, qui refuse depuis le ticket #25 les commandes et réponses inconnues, rejettera `connecter_adresse`, `version_dll` et `instruments_fd9` : l'application et les ponts se mettent à jour ensemble.
