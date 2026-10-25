# Proyectos multi-cuenta (git, AWS, GCP) por directorio

Patron para trabajar varios clientes a la vez, cada uno con su correo de git,
su repo remoto, su cuenta AWS y su proyecto GCP — **cambiando de identidad
automaticamente segun la carpeta**, sin configurar nada a mano cada vez.

## Estructura de carpetas

```
~/work/
├── cliente-a/        # correo A, repo A, AWS perfil A, GCP config A
│   └── .envrc        # activa el entorno de A al entrar
├── cliente-b/
│   └── .envrc
└── personal/
    └── .envrc
```

El pegamento es **direnv** (ya instalado): al entrar a una carpeta, lee su
`.envrc` y activa las variables; al salir, las desactiva.

## 1. Git: identidad por carpeta (nativo, via includeIf)

En `~/.gitconfig` (global) se delega la identidad por subarbol. devlab gestiona
el gitconfig base; añade bloques `includeIf` por cliente:

```gitconfig
[includeIf "gitdir:~/work/cliente-a/"]
    path = ~/work/cliente-a/.gitconfig
[includeIf "gitdir:~/work/cliente-b/"]
    path = ~/work/cliente-b/.gitconfig
```

Y en `~/work/cliente-a/.gitconfig`:

```gitconfig
[user]
    name = Tu Nombre
    email = tu-correo@cliente-a.com
```

Git usa automaticamente ese correo para cualquier repo bajo `~/work/cliente-a/`.

## 2. SSH: clave distinta por cliente (para repos de GitHub separados)

Genera una clave por cliente:

```
ssh-keygen -t ed25519 -C "cliente-a" -f ~/.ssh/cliente-a
```

En `~/.ssh/config` define un host alias por cliente:

```
Host github-cliente-a
    HostName github.com
    User git
    IdentityFile ~/.ssh/cliente-a
    IdentitiesOnly yes
```

Y clonas usando el alias:
`git clone git@github-cliente-a:org/repo.git`

Asi cada cliente usa su propia clave SSH / cuenta de GitHub.

## 3. AWS CLI: un perfil por cliente

En `~/.aws/config`:

```ini
[profile cliente-a]
sso_start_url = https://cliente-a.awsapps.com/start
sso_region = us-east-1
region = us-east-1

[profile cliente-b]
region = us-west-2
```

Login por cliente: `aws sso login --profile cliente-a` (o `aws configure --profile cliente-a`).

## 4. GCP: una configuracion por cliente

```
gcloud config configurations create cliente-a
gcloud config configurations activate cliente-a
gcloud auth login
gcloud config set project proyecto-a
```

Se cambia con `gcloud config configurations activate <nombre>`.

## 5. El `.envrc` que lo activa todo (direnv)

En `~/work/cliente-a/.envrc`:

```sh
# AWS: usa el perfil de este cliente en todos los comandos aws
export AWS_PROFILE=cliente-a

# GCP: activa la configuracion de este cliente
export CLOUDSDK_ACTIVE_CONFIG_NAME=cliente-a
export GOOGLE_CLOUD_PROJECT=proyecto-a
```

La primera vez, direnv pide permiso: `direnv allow` dentro de la carpeta.
A partir de ahi, al entrar a `~/work/cliente-a/` (o cualquier subcarpeta) se
activan AWS perfil A y GCP config A; al salir, se desactivan. git usa el correo
de A por el `includeIf`.

## Checklist para un cliente nuevo

```
mkdir -p ~/work/cliente-x && cd ~/work/cliente-x
# 1. git: anadir includeIf en ~/.gitconfig + crear ~/work/cliente-x/.gitconfig
# 2. ssh: ssh-keygen -f ~/.ssh/cliente-x  + host alias en ~/.ssh/config
# 3. aws: perfil en ~/.aws/config
# 4. gcp: gcloud config configurations create cliente-x
# 5. direnv: crear .envrc (AWS_PROFILE, CLOUDSDK_ACTIVE_CONFIG_NAME) + direnv allow
```

## Nota de seguridad

Los `.envrc` NO deben contener secretos (claves, tokens) — solo nombres de
perfil/proyecto. Los secretos viven en el keyring de cada herramienta (AWS SSO,
gcloud auth) o cifrados con sops. Si un `.envrc` necesitara un secreto, cargalo
desde un archivo fuera del repo o con sops, nunca en texto plano versionado.
