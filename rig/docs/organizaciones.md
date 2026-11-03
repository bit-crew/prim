# Organizaciones — una cuenta de git por carpeta (prim org)

Una **organización** es una cuenta de trabajo (GitHub, Bitbucket, ...) con su
propia identidad en la máquina: su carpeta bajo `~/work`, su clave SSH y su
entorno direnv. Al entrar a la carpeta, todo `git` de esa sesión usa la cuenta
de esa organización; al salir, vuelve a la identidad por defecto.

La gestión la hace el binario `prim` (rig lo instala en `~/.local/bin`):

```
prim org new <nombre>         # crea/actualiza la organizacion (idempotente)
prim org secret <nombre> K=V  # agrega o actualiza una variable en .env.local
prim org list                 # lista las organizaciones
```

## Crear una organizacion

```
prim org new globaltask
```

Hace los seis pasos, sin pisar nada existente (puede re-ejecutarse):

1. Crea `~/work/globaltask`.
2. Crea `.envrc` (`dotenv_if_exists .env.local`).
3. Genera la clave SSH de la org: `~/.ssh/id_ed25519_globaltask`
   (ed25519, sin passphrase; una por org y por máquina, nunca reutilizada).
4. Crea `.env.local` con permisos `0600`, vacío si no existía.
5. Agrega `GIT_SSH_COMMAND="ssh -i ~/.ssh/id_ed25519_globaltask -o IdentitiesOnly=yes"`
   solo si no estaba.
6. `direnv allow` sobre la carpeta.

Al terminar imprime la **clave pública**: registrala en el proveedor como
deploy key de la org (o en la cuenta de GitHub/Bitbucket de esa organización).
Con eso, dentro de la carpeta:

```
cd ~/work/globaltask
git clone git@github.com:globaltask/repo.git   # usa la clave de ESTA org
```

## Secretos por organizacion

Los secretos (tokens, API keys) viven en `~/work/<org>/.env.local`, nunca en
`.envrc` ni versionados. `prim org secret` modifica el archivo existente sin
tocar las demás variables y re-autoriza direnv:

```
prim org secret globaltask TURSO_API_KEY=xyz     # agrega
prim org secret globaltask TURSO_API_KEY=abc     # actualiza
```

direnv los carga al entrar a la carpeta (o con `direnv reload` si ya estás dentro).

## Aterrizaje en ~/work

Al instalar, rig crea `~/work`. Al abrir una consola nueva o entrar por SSH sin
estar en ningún proyecto, la sesión arranca directamente en `~/work` (guarda
`PRIM_LANDING` en el zshrc: no re-mueve shells anidadas ni panes que ya están
en otra carpeta).

## Qué sigue siendo manual

Identidad git por carpeta (`includeIf` + `.gitconfig` por org), alias de host
SSH y perfiles AWS/GCP por cliente: ver [multi-cuenta.md](./multi-cuenta.md).
