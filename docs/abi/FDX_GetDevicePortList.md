# FDX_GetDevicePortList

**En clair** : demande à la DLL la liste des instruments qu'elle voit (branchés en USB ou, peut-être, sur le réseau). C'est le palier Détection. Une liste vide n'est pas une erreur : elle veut dire « rien de branché ».

## Signature

```c
int32_t __stdcall FDX_GetDevicePortList(RE_FDX_Port44 *out_ports,  /* tableau, peut être NULL */
                                        uint32_t *out_count,       /* obligatoire */
                                        uint32_t capacity_ports);  /* x86 : ret 12 */
typedef struct { int32_t transport_code; uint8_t opaque[40]; } RE_FDX_Port44; /* 44 octets */
```

## Ce qui est confirmé

- Trois arguments : le tableau à remplir, un pointeur vers le compteur, la capacité du tableau **en nombre d'entrées** (pas en octets).
- Capacité supérieure à 100 : erreur -9992. Compteur nul (`out_count == NULL`) : erreur -9992. Capacité 0 acceptée.
- **Appel en deux temps**, utilisé par MY-CT1 :
  1. `(NULL, &count, 0)` : la DLL écrit seulement le nombre d'instruments trouvés ;
  2. `(tableau, &count, 100)` : la DLL copie les entrées.
- Si la capacité est trop petite, la DLL écrit le nombre trouvé mais ne copie pas le tableau : il faut toujours relire `count` avant d'utiliser les entrées.
- Chaque entrée fait **44 octets** (pas de copie vérifié dans la DLL). EIZO réserve 20 entrées (880 octets), passe la capacité 20, et utilise la première entrée pour se connecter : preuve indépendante en x64.
- **Refus pendant une connexion active** : si un instrument est connecté et que l'état interne n'est pas 1, 2, 3, 7 ou 8, erreur -9986 (« device is connected. »).

## Contenu d'une entrée

- `+0` : entier qui ne peut valoir que 0 ou 1 (contrôlé par `FDX_Connect`) : **confirmé**. Qu'il désigne le moyen de liaison (USB ou réseau) est **supposé** : la DLL importe aussi la bibliothèque réseau Windows.
- `+4` : chaîne de caractères terminée par zéro, lue par `FDX_Connect` : **confirmé**. Son contenu (chemin de périphérique, nom de port…) et sa longueur maximale ne sont pas établis.
- `+40` : nombre affiché par MY-CT1 dans sa liste : **confirmé** comme affichage. Qu'il s'agisse du numéro de série est **supposé**.
- Le reste est opaque : le pont doit conserver les 44 octets intacts et les rendre tels quels à `FDX_Connect`.

## Pour le pont

- Toujours faire l'appel en deux temps, avec une capacité ≤ 100.
- Présenter chaque entrée à l'application comme un identifiant opaque, plus un libellé (le nombre à `+40`, marqué supposé).
- Appeler avant `FDX_Connect` ; ne pas rappeler pendant une mesure.

## À vérifier sur l'instrument

- La valeur de `+0` pour le MYIRO-1 en USB (attendu : 0 ou 1).
- Le contenu de la chaîne à `+4`, et si le nombre à `+40` est bien le n° de série (le poste connaît 10002006).
- Le comportement avec deux instruments, ou avec le MYIRO-1 en réseau.

## Preuves (locales)

- `fdx-x86/exports/FDX_GetDevicePortList.asm.txt` : capacité ≤ 100 à `0x10035969`, compteur obligatoire à `0x1003597f`, `ret 0xc`.
- `fdx-x86/internes/GetDevicePortList.asm.txt` : contrôle d'état à `0x1001c15a..0x1001c183`, refus « device is connected. » à `0x1001c1c4`, pas de 44 octets à `0x1001c282`.
- `preuves/myct1/enumeration-deux-passes.asm.txt` : appels à `0x402773` et `0x402800`.
- `preuves/eizo-x64/enumeration.asm.txt`.
