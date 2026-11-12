# Ambientes de teste do revisor

Consulte a linha do ambiente em que o comportamento do PR vive. O que o PR toca decide o ambiente: `#[cfg(windows)]`, hotkeys, `prtsc_hook`, `borderless`, bandeja no Windows; `#[cfg(target_os = "linux")]`, Wayland, portal, X11 em Linux; `#[cfg(target_os = "macos")]` no Mac; o resto roda em qualquer um.

| Ambiente | Nesta máquina | Alcança | Não alcança |
|---|---|---|---|
| Windows (host) | Windows 10 Pro, 2 monitores 1920x1080 a 100%: o primário em 0,0 e o outro à esquerda, em x=-1920 (origem negativa) | build, testes, `--screenshot`, o app em execução conduzido por `SendInput` (teclas, mouse, janelas, arquivo gravado), captura em 2 monitores | escalas diferentes entre monitores, outro sistema |
| Linux X11 | container Ubuntu 24.04 com Xvfb (Docker Desktop) | `--screenshot`, residente sem bandeja, janela e foco com openbox | bandeja, atalho global de um desktop real; vários monitores (ver abaixo) |
| Linux Wayland | container com sway headless e mako | `--screenshot`, residente com bandeja (swaybar), aviso como notificação, multi-saída | portal de um desktop real, GNOME e KDE |
| macOS | nenhum | só compilar: `check.sh` roda clippy para `aarch64-apple-darwin`; o CI roda os testes | tudo que executa |
| Máquinas do usuário | Kubuntu (KDE, Wayland), GNOME Wayland (onde aparece o erro de protocolo do #13), os monitores dele | o que só desktop real mostra | disponível só com o usuário no teclado |

## Windows (host)

- Build na branch do PR: `cargo build --release > <shots>/build.log 2>&1`, com o `CARGO_TARGET_DIR` da skill. Build debug é programa de console: redirecione a saída ou a janela nova rouba o foco.
- Telas: `takeshot --page <p> --theme <t> --mode <m> --lang pt --captures-dir design/screens --screenshot out.png`, nos quatro visuais; `scripts/screenshots.sh` renderiza tudo para `shots/`.
- Ponta a ponta, por você: avise o usuário por quanto tempo vai usar mouse e teclado, copie o `settings.json` (seção Limites da skill) e conduza o cenário no exe da `main` (vermelho) e no da branch (verde) com [windows-ui.ps1](windows-ui.ps1), que manda teclas e cliques reais por `SendInput` e só envia com a janela do app em primeiro plano. O Takeshot não é um app instalado, então o `computer-use` não consegue ser autorizado para ele. Só rode com nenhum Takeshot do usuário aberto: o app em execução registra atalhos globais e grava ajustes.
- Inicie sempre com `--captures-dir <repo>\design\screens`: a Home mostra a pasta de capturas do usuário, e ela volta a aparecer quando a janela reabre. Se você encerrar à força o processo de uma captura, a janela principal fica escondida; abrir o exe de novo a traz de volta.
- Screenshot da janela com `Shot`; leia o PNG para conferir. Atalho gravado vira `hotkey_area` (e `hotkey_full`) em `settings.json`, que dá para ler para confirmar. Medida repetida (30 tentativas de gravar um atalho com repetição automática simulada) é um laço no próprio PowerShell: `Key-Down` repetido do último modificador a cada ~33 ms.
- Coordenada de clique que muda com o estado: em Ajustes > Atalhos, com um atalho diferente do padrão a linha mostra "Change" (x≈935) e "Restore default" (x≈1033); com o padrão, só "Change" (x≈1056). Clicar sempre no mesmo ponto aperta "Restore default" nas rodadas alternadas e parece perda de tecla (12/11, #9). Leia o PNG de `Shot` antes de concluir que o app perdeu a tecla.
- Captura, com a tela do usuário fora das imagens: [windows-monitores.ps1](windows-monitores.ps1) (dot-source) traz `Start-Backdrop` ([windows-pano-de-fundo.ps1](windows-pano-de-fundo.ps1): uma janela de cor sólida em cada monitor, o primário vermelho e o outro azul), `Wins`/`Wins-Text` (janelas visíveis do pid com posição, tamanho e topmost), `Hwnd-Of`/`Focus-Hwnd` (foca uma janela do app por Win32), `Drag`, `Chord`, `Top-At` (quem recebe um clique num ponto) e `Backup-Settings`/`Restore-Settings`. O mouse por `SendInput` chega na sobreposição da captura (12/11, #10); clique por PostMessage não.
- A cópia de tela (`CopyFromScreen`) de uma captura em tela cheia congela depois do primeiro arrasto, na `main` também: o quadro apresentado direto pela GPU não aparece na cópia. Não conclua nada por pixel depois de interagir. Observe o estado: qual janela salva com `Ctrl+S` ao focá-la (só a que tem área salva), o PNG gravado (tamanho e cor do centro com o pano de fundo), onde abre cada janela e se um dialogo é `topmost` e recebe o clique (`Top-At` no centro dele).
- O PNG de uma área arrastada com entrada sintética sai maior que o arrasto (200x100 vira 228x128, 400x300 vira 456x356), igual na `main` e sem explicação ainda; compare o tamanho entre `main` e PR em vez de esperar o valor exato.
- O `settings.json` que o teste escreve vai em UTF-8 sem BOM: o app lê com `serde_json`, que recusa o BOM, e cai nos padrões em silêncio (o `Set-Content -Encoding UTF8` do PowerShell 5.1 grava BOM). Escreva com `[System.IO.File]::WriteAllText(path, json, (New-Object System.Text.UTF8Encoding $false))`, como `Set-TestSettings` faz.

## Linux (containers)

Do checkout principal, com o sha do PR já buscado:

1. `scripts/linux-packages.sh <sha>` constrói `.deb` e `.AppImage` a partir de `git archive <sha>` na imagem do CI (`scripts/ci-linux`), com volumes nomeados que mantêm o build quente.
2. `scripts/linux-smoke.sh` abre os pacotes em containers limpos (X11, Wayland) e salva as imagens em `target/packages-linux/smoke`; `scripts/linux-smoke.sh resident` confere o ciclo da janela do residente.
3. **Cenário do PR:** copie o padrão de `run()` em `scripts/linux-smoke.sh` e de `scripts/container/resident-check.sh` para um script em pasta temporária (fora do repo), monte-o como `/t:ro` e conduza o app:
   - X11: `xdotool` (teclas, mouse), `wmctrl`, `xprop`, captura do display com ImageMagick `import -window root`.
   - Wayland (sway): `swaymsg` (árvore, foco, saídas), `wtype` para teclas, `grim` para captura.
   - Várias telas: o xcap enumera por RandR (`GetMonitors`, um monitor por output), então `Xvfb` com `-screen 1` ou `+xinerama` não serve (uma tela só, e `--setmonitor` não cria output). No X11 use Xorg com `xserver-xorg-video-dummy` (`Xorg :99 -noreset -config xorg.conf -novtswitch -sharevts +extension RANDR` com `Driver "dummy"` e `Virtual 4000 2000`): o `xrandr` lista `DUMMY0` a `DUMMY15`, só o primeiro conectado; ligar o segundo (`xrandr --output DUMMY1 ...`) ainda não foi testado. Wayland: `WLR_HEADLESS_OUTPUTS=2` e conferência com `swaymsg -t get_outputs`. A opção "capturar todas as telas" fica oculta no Wayland.
   - Portal de captura no Wayland: `xdg-desktop-portal` com o backend `xdg-desktop-portal-wlr` e um barramento `dbus-run-session`, como `resident-check.sh` já faz para o mako.
4. Receitas de multi-tela e de portal ainda não foram rodadas nesta máquina. Na primeira vez que uma delas servir a um cenário, confirme com o comando de conferência antes de confiar no resultado e corrija esta seção com o que funcionou.

## macOS

Sem Mac aqui. `scripts/check.sh --linux` compila e linta o código de macOS e o CI roda os testes. Tudo que executa vai para "Não verificado" do relatório com os passos aplicáveis de `docs/testing-macos.md`, e o veredito nunca afirma que funciona no Mac.

## Roteiro para o usuário

Para o que nenhum ambiente acima alcança (macOS, Wayland real, vários monitores, interação com mouse), entregue exe e roteiro:

1. Compile release e copie o exe do PR para `shots/pr-<n>/` com o SHA no nome. Compile também o da `main` (`git switch --detach origin/main`, compile, copie, `git switch pr-<n>`; reaproveite o de outra pasta se o SHA da `main` for o mesmo) e confira que os dois exes têm sha256 diferentes. Se o PR mexe em `Cargo.toml`, `build.rs`, `packaging/`, `assets/brand/` ou `.github/`, o CI gera os instaladores: baixe o artifact com `gh run download` em vez de compilar e confira que ele serve. Linux: deixe o AppImage ou o `.deb` ao lado.
2. Copie `%APPDATA%\takeshot\settings.json` para `shots/` antes; restaure só sem nenhum Takeshot rodando.
3. Escreva o roteiro no formato de `.opencode/memory/process.md`, seção 4, com os comandos prontos para o PowerShell:
   - pré-requisitos: qual exe, fechar o Takeshot da bandeja, o comando, o aviso do backup;
   - passo A: a `main`, como linha de base;
   - passos numerados, cada um com a ação concreta e o resultado visível esperado (passo vago o usuário devolve);
   - plano B, se algo não abrir;
   - "O que me mandar": OK ou falhou por passo, prints, monitores e escala.
