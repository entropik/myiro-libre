# FD9_GetStatus

**En clair** : donne l'état courant de la session (où en est le SDK, si une feuille est engagée…), sous forme de 8 nombres. **Elle ne pose pas de question à l'instrument** : elle recopie le dernier état reçu par la DLL. Le pont l'appelle une fois après la connexion, pour le journal.

## Signature

```c
typedef struct { uint32_t mots[8]; } tFD9_Status;   /* 32 octets */

int32_t __stdcall FD9_GetStatus(tFD9_Status *etat);  /* x86 : ret 4 */
```

## Ce qui est confirmé

- **Contrôle de l'export** (`fd9-x86` `0x100b14d0`) : `etat` non nul, sinon **1001**.
- Fonction de travail `0x1008bee0` : hors connexion, code interne 8077, public **1102** (`0x1008beed..0x1008bef6`).
- Connectée, elle recopie un bloc gardé par l'objet de communication, entre deux appels qui encadrent la lecture (`0x1008bf09..0x1008bf58`), puis remplit la structure. **Aucune commande n'est envoyée.**

| Position | Contenu | Niveau |
|---|---|---|
| +0x00, +0x04, +0x0c | trois nombres du bloc reçu de l'instrument | position confirmée, sens non établi |
| +0x08 | une variable de la session (`0x10221e98`) | position confirmée, sens non établi |
| +0x10 | **état du SDK** (`FD9_eSDKStatus`, même variable que celle testée par les autres fonctions, `0x1022235c`) | confirmé |
| +0x14 | une valeur reçue ramenée à 0 ou 1 (0 et 1 → 0 ; 2 et 3 → 1) ; **non écrit** pour une autre valeur reçue (`0x1008bf7b..0x1008bf9b`) | confirmé ; « état du papier » supposé |
| +0x18 | toujours 0 | confirmé |
| +0x1c | un nombre d'éléments d'une liste de la DLL | position confirmée, sens non établi |

## Propriété de la mémoire

- La structure appartient à l'appelant ; la DLL y écrit 32 octets pendant l'appel et ne garde pas son adresse (confirmé).
- Le champ +0x14 peut rester tel que l'appelant l'a laissé : la structure doit être mise à zéro avant l'appel.

## Ce qui est supposé

- Les deux appels qui encadrent la recopie prennent et rendent un verrou : la lecture peut se faire pendant qu'un fil de la DLL reçoit des messages.
- +0x14 = 1 : une feuille est engagée.

## Pour le pont

- Structure à zéro avant l'appel ; garder les 8 nombres bruts dans le journal de session.
- Ne jamais faire avancer la session sur `FD9_GetStatus` : seuls les événements du rappel et les codes de retour la font avancer.

## À vérifier sur l'instrument

- Les 8 nombres après la connexion, puis feuille engagée (essai E1).

## Preuves (locales)

- `fd9-x86/exports/FD9_GetStatus.asm.txt` (`ret 4`) ; `fd9-x64/exports/FD9_GetStatus.asm.txt`.
- Fonction de travail : `desassemble.py fd9-x86 --address 0x1008bee0`.
- Taille : `retroanalyse/logiciels/fd-s2w.md` § 4.3.
