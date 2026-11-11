# rig TODO

Pendientes operativos del VPS (`rig/`). Los items que pertenecen a una
aplicación concreta viven en el repo de esa aplicación; acá queda lo que
toca **al VPS como host**.

Estado de la pila WorkSpaces (app + cliente + KasmVNC): repo `gt`, rama
`feat/workspaces`, doc de feature en `gt/odd/tasks/workspaces-android.md`.

---

## 1. Teclado desde la tablet — bloqueante si hay que escribir 🔴

**Qué pasa.** El escritorio se ve en Chrome (Android) dentro de KasmVNC. Chrome
en Android no manda el teclado de hardware a un canvas VNC (no es un `<input>`),
y KasmVNC no trae teclado en pantalla por defecto. hoy se puede **mirar** el
escritorio, no **escribir** en él.

**Qué hay que decidir.** ¿El escritorio corporativo es solo consulta, o hay que
trabajar sobre él? Si hay que escribir, hay que meter un teclado en pantalla en
la sesión (p. ej. `xvkbd` o `matchbox-keyboard` en el flake) o cambiar de
cliente.

**Dónde.** `gt/workspaces/flake.nix` (paquetes de la sesión) y
`gt/workspaces/scripts/ws-session.sh` (arranque de la sesión).

**Cómo se verifica.** Conectar desde la tablet y escribir en una app del
escritorio (p. ej. el navegador corporativo).

## 2. Túnel DCV / PCoIP hacia el WorkSpace — nunca probado

**Qué pasa.** El cliente murió siempre en el broker (HTTP 400 / `ErrorCode: 6001`),
así que **nunca llegó a conectarse** al workspace. El firewall que se instaló
en `rig/host/harden-vps.sh` (tabla `vps_hardening`) verificó egress general, pero
no el tráfico real del túnel.

**Qué hay que probar.** DCV: 443 TCP/UDP. PCoIP: 4172/4195 TCP/UDP. Con el
broker funcionando, ver en el log del cliente que la sesión levanta.

**Cómo se verifica.** `nft list table inet vps_hardening` + la sesión del cliente
conectada de verdad.

## 3. `WS_PASSWORD`: ¿el broker pide contraseña o no?

**Qué pasa.** `gt/workspaces/scripts/ws-password.sh` está implementado y
cableado (dispara con la línea de provider del log del cliente), pero **nunca se
ejercitó**: el broker rechaza antes de pedir nada. Además el directorio reporta
`provider SAML_IAM` (SSO), o sea que puede que el prompt de contraseña **no
exista nunca** para este workspace y `WS_PASSWORD` quede como código inerte.

**Qué hay que hacer.** Cuando el broker acepte, ver si aparece el prompt. Si
aparece, ajustar `GT_WS_PASSWORD_TABS` hasta que el foco caiga en el campo de
contraseña. Nunca registrar el valor.

## 4. Memoria del host con la sesión activa

**Qué pasa.** El VPS tiene 3.8 GB de RAM. Con el cliente + el webview CEF ya
corriendo se usan ~500 MB. Al conectarse el escritorio, el cliente decodifica
video y sube el consumo.

**Cómo se vigila.** `free -m` y `ps -eo rss,comm --sort=-rss | head`. El swap de
2 GB (`/swapfile`, `vm.swappiness=10`) está como red de seguridad; si el
consumo crece de forma sostenida, la sesión es lo primero que hay que mirar.

---

## Dependencia externa (no la podemos resolver desde el VPS)

El broker de WorkSpaces rechaza la autenticación del cliente Linux:

```
WSBrokerClient::Authenticate failed
ErrorCode: 6001, ErrorMessage: User Authentication failed
```

Del lado nuestro el flujo llega bien: assertion SAML válida (STS la acepta, rol
`arn:aws:iam::367222308331:role/Saml2EX`, principal
`arn:aws:iam::367222308331:saml-provider/OktaExternos`), ACS 200 y handoff
nativo entregado al cliente.

Del lado de la organización hacen falta dos cosas:

1. Habilitar **Linux** en *WorkSpaces → Directory `d-9067932154` → Settings →
   Other clients*. Sin eso el cliente Linux no se autentica (AWS re:Post
   documenta exactamente este síntoma).
2. Confirmar que la app Okta `amazonworkspacesexternos` tiene el rol de la
   cuenta que administra el WorkSpace. El rol `Saml2EX` vive en la cuenta
   `367222308331` y **no** tiene permisos `workspaces:*`; puede ser una cuenta
   distinta de la del directorio.
