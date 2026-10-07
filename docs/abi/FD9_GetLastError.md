# FD9_GetLastError

**En clair** : demande à la DLL le code de la dernière erreur, tel qu'elle l'a reçu en interne. Ne parle pas à l'instrument. C'est le seul appel du palier Version : FD9SDK n'a pas de fonction publique qui donne sa version.

## Signature

```c
int32_t __stdcall FD9_GetLastError(void);   /* x86 : ret (aucun argument) */
```

## Ce qui est confirmé

- Aucun argument ; la fonction se contente de relire une variable globale (`fd9-x86` `0x100b0e50`, `fd9-x64` `0x1800d0280`).
- **Elle rend le code interne d'origine, pas le code public** : la fonction de regroupement mémorise le code reçu avant de chercher son groupe (`fd9-x86` `0x100b43d5`, `fd9-x64` `0x1800d4517`). Exemple : quand la connexion TCP de `FD9_Connect` échoue, l'export renvoie 1003 et `FD9_GetLastError` rend ensuite 4001 (valeurs confirmées ; que 4001 corresponde à une adresse injoignable est supposé).
- Chaque export remplace ce code, y compris par 0 en cas de succès.
- La seule autre fonction de version est `JIG_GetSDKVersion`, un export de maintenance : il reste hors de la liste blanche.

## Ce qui est supposé

- Avant tout autre appel, la valeur est 0 (variable globale non initialisée par le code, donc à zéro au chargement).

## Propriété de la mémoire

- Aucun pointeur échangé : la fonction rend un entier (confirmé, `0x100b0e50`).

## Le palier Version du FD-9

Sans fonction publique de version, le palier Version du pont fait trois choses, sans parler à l'instrument :

1. lire la version dans la ressource du fichier DLL (`FileVersion` : **1.3.2.3** en x86, **1.3.1.5** en x64), avec l'outil de version de Windows, sans appel à la DLL ;
2. calculer l'empreinte SHA-256 de la DLL, qui identifie la version sans ambiguïté ;
3. charger la DLL, résoudre les six exports de la liste blanche, et appeler `FD9_GetLastError` : un retour prouve que la DLL est chargée et répond.

La version écrite dans le code du SDK (`{1, 0x20, 3}` en 1.3.2.3) n'est lisible qu'après la connexion, par [FD9_GetSystemInfo](FD9_GetSystemInfo.md).

## Pour le pont

- Archiver le code interne de `FD9_GetLastError` à côté du code public après chaque échec.
- Ne pas déduire de cet appel que la convention `__stdcall` est la bonne : sans argument, il ne la met pas à l'épreuve. C'est `FD9_GetDeviceList` qui la vérifie.

## Preuves (locales)

- `fd9-x86/exports/FD9_GetLastError.asm.txt` et `fd9-x64/exports/FD9_GetLastError.asm.txt`.
- Regroupement : `desassemble.py fd9-x86 --address 0x100b43b0` et `fd9-x64 --address 0x1800d4500`.
- Ressource de version : `retroanalyse/logiciels/fd-s2w.md` § 2.1.
