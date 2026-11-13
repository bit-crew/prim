# Activación no destructiva del rig (dotfiles)

## Specs

- S1: "prim NUNCA sobreescribe un dotfile existente: escribe solo los que faltan y lista los divergentes para que vos los merges."
- S2: "La fuente de verdad es la config que ya tenés: tu version gana; la del rig queda archivada."
- S3: "Los dotfiles que el rig crea de cero son archivos reales editables (no symlinks al store)."
- S4: "prim te reporta el diff entre tu version y la del rig."
- S5: "Si hay un ~/.gitconfig con usuario global, prim lo adopta sin preguntar."
- S6: "Los includeIf de un ~/.gitconfig preexistente se migran a ~/.config/git/orgs.config."
- S7: "El harness no se toca en esta pasada."

## Tasks

- T1 (S1-S4): modulo `src/dotfiles.rs` con manifiesto, snapshot, merge, diff. Route: inline. Commit: feat(prim).
- T2 (S5-S6): bootstrap adopta la identidad existente y migra la cadena de orgs. Route: inline. Commit: feat(prim).
- T3 (S1-S4): integrar el merge en la activacion y la desinstalacion de `src/nix.rs`. Route: inline. Commit: feat(prim).
- T4 (S1): el flake no pisa: `mkIf` (no `optionalAttrs`) en las opciones por perfil. Route: inline. Commit: fix(rig).
- T5 (S1): las unidades systemd como archivos crudos (home.file) + symlink del timer por activacion; home-manager no puede importar un unit crudo. Route: inline. Commit: fix(rig).
- T6: publicar en main y reinstalar prim (el usuario). Route: user.

## Log

- L1 (2026-11-12, request, verbatim): "Porqeu putas sigue separado Rig de files?"
- L2 (2026-11-12, request, verbatim): "Y esto noe s solo por git, por ejemplo yo tengo un monton de cosas ya en cuato a harness configuradas, no es viable para mi que prim me las borre todas porque si"
- L3 (2026-11-12, request, verbatim): "Hagamaso un analis más profindo, por cierto porque no haces preguntas par aqeu lleguemso al mejor diseño posible"
- L4 (2026-11-12, decision): politica "merge, never clobber". Mecanismo elegido: prim protege alrededor de home-manager (snapshot -> switch -> restore + reporte), no un guard en el flake.
- L5 (2026-11-12, decision): los dotfiles nuevos son archivos reales editables, no symlinks al store.
- L6 (2026-11-12, decision): el harness no se toca en esta pasada.
- L7 (2026-11-12, discovery): home-manager reemplaza cada ruta administrada por un symlink al store (`home-files`); con `-b` la guarda como `<archivo>.prim-backup`, y si ese backup ya existe la activacion FALLA. `HOME_MANAGER_BACKUP_OVERWRITE=1` lo evita.
- L8 (2026-11-12, discovery): el perfil `vps` del flake NO era evaluable: `lib.optionalAttrs` declaraba opciones vacias ("the option ... was accessed but has no value defined") y `systemd.user.services.<n>.unit` recibia un string donde HM exige una estructura. El perfil `dc1` tampoco (mismo patron en termux.nix). Ambos compilan ahora.
- L9 (2026-11-12, discovery): en esta maquina HM nunca se activo: todos los dotfiles son archivos regulares de la era chezmoi. El manifiesto vps son 19 rutas.
- L10 (2026-11-12, decision): las unidades del inbox se.placean con `home.file` (fuente unica: los archivos crudos de rig/vps/) y el timer se arma con un symlink en `home.activation`; home-manager no puede importar un unit crudo (lo renderiza desde una estructura).
- L11 (2026-11-12, evidence): `nix build .#homeConfigurations.{vps,dc1}.activationPackage` OK para ambos perfiles; manifiesto vps = 19 rutas.
