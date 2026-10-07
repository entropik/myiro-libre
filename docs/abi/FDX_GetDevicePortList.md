# FDX_GetDevicePortList

**En clair** : demande à la DLL la liste des instruments qu'elle voit, branchés en USB ou présents sur le réseau. C'est le palier Détection. Une liste vide n'est pas une erreur : elle veut dire « rien de branché ».

## Signature

```c
int32_t __stdcall FDX_GetDevicePortList(FDX_PortInfo *out_ports,  /* tableau, peut être NULL */
                                        uint32_t *out_count,      /* obligatoire */
                                        uint32_t capacity_ports); /* x86 : ret 12 */
```

## Ce qui est confirmé

- Trois arguments : le tableau à remplir, un pointeur vers le compteur, la capacité du tableau **en nombre d'entrées** (pas en octets).
- Capacité supérieure à 100 : erreur -9992. Compteur nul : erreur -9992. Capacité 0 acceptée.
- **Appel en deux temps**, utilisé par MY-CT1 : `(NULL, &count, 0)` pour connaître le nombre, puis `(tableau, &count, 100)` pour les entrées.
- Si la capacité est trop petite, la DLL écrit le nombre trouvé mais ne copie pas le tableau : toujours relire `count` avant d'utiliser les entrées.
- **Refus pendant une connexion occupée** : si un instrument est connecté et que l'état interne n'est pas 1, 2, 3, 7 ou 8, erreur -9986.
- **Détection USB** : VID `132B`, PID `210D` ou `210F`, n° de série USB commençant par `ACJ1` ou `9C1D` (les deux codes produit de la famille MYIRO-1). Le PID `210D` est aussi celui du FD-9 et du MYIRO-9, que leur SDK distingue par les préfixes `9C1A` et `A8AN` : **le PID seul n'identifie pas le modèle**, le préfixe du n° de série si.
- **Détection réseau** : diffusion UDP du message « FDXSDK » vers le port 49152.

## Contenu d'une entrée (44 octets, confirmé par les SDK Mac non dépouillés)

| Position | Taille | Contenu |
|---|---|---|
| `+0x00` | i32 | liaison : **0 = réseau (TCP), 1 = USB** |
| `+0x04` | 33 octets max | nom du port, terminé par zéro : `COMn` sous Windows, `/dev/cu.usbmodem…` sur Mac, adresse IP en réseau |
| `+0x25..+0x27` | 3 octets | remplissage |
| `+0x28` | u32 | **n° de série** de l'instrument, 8 chiffres (en USB : chiffres du n° de série USB après le préfixe) |

## Pour le pont

- Toujours faire l'appel en deux temps, avec une capacité ≤ 100.
- Présenter chaque entrée à l'application avec sa liaison, son nom de port et son n° de série ; rendre les 44 octets intacts à `FDX_Connect`.
- Appeler avant `FDX_Connect` ; ne pas rappeler pendant une mesure.
- Ne jamais ouvrir un périphérique Konica Minolta de PID `210E` : aucun SDK ne le cherche, il s'agit probablement du mode de mise à jour du micrologiciel (`retroanalyse/logiciels/firmware-fd9.md`).

## Vérifié sur l'instrument

Le 7 octobre 2026, MYIRO-1 branché en USB, DLL 1.0.1.0 en x86 et en x64 (test `palier_detection_avec_la_vraie_dll`) : une seule entrée, liaison 1 (USB), port `COM3`, n° de série égal à celui que Windows connaît. L'appel en deux temps et la découpe de l'entrée sont donc **confirmés à l'exécution**. Le FD-9 du réseau n'apparaît pas : FDXSDK ne cherche que la famille MYIRO-1. Aucun fichier n'a été créé.

## Reste à vérifier

- Le comportement instrument débranché (liste vide attendue), avec deux instruments, ou avec le MYIRO-1 en Wi-Fi.

## Preuves (locales)

- `fdx-x86/exports/FDX_GetDevicePortList.asm.txt` : capacité ≤ 100 à `0x10035969`, compteur obligatoire à `0x1003597f`, `ret 0xc`.
- `fdx-x86/internes/GetDevicePortList.asm.txt` : contrôle d'état à `0x1001c15a..0x1001c183`, pas de 44 octets à `0x1001c282`.
- `preuves/myct1/enumeration-deux-passes.asm.txt` : appels à `0x402773` et `0x402800`.
- `retroanalyse/logiciels/my-ct1.md` § 5.1 : découpe de l'entrée, USB et réseau ; `retroanalyse/logiciels/fd-s2w.md` et `firmware-fd9.md` : PID partagé et PID `210E`.
