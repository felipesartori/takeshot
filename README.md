# Takeshot

Ferramenta de captura de tela para Windows, macOS e Linux: captura instantânea com anotação e um editor para fundos, cantos arredondados, sombra e marca d'água.

![Editor](docs/editor.png)

## O que faz

**Captura**
- Seleção de área com lupa de pixels, coordenadas e cor (tecla `C` copia o hex da cor sob o cursor).
- Alças para ajustar a seleção, setas movem, `Shift`+setas redimensionam, `Ctrl+A` pega a tela inteira.
- Opção de lembrar a última área selecionada.
- Tela inteira direto no editor e captura com temporizador de 3 segundos.

**Ferramentas** (barra flutuante sob a seleção)
- Caneta, linha, seta, retângulo, círculo/elipse, marcador, texto, borrar ou pixelar, numeração de passos.
- `Shift` enquanto arrasta: linhas em ângulos de 45°, quadrados e círculos perfeitos, traço livre reto.
- `Alt` enquanto arrasta: retângulo, círculo e desfoque a partir do centro.
- Paleta de cores com cor personalizada, 5 espessuras (a roda do mouse ajusta), retângulo e círculo preenchidos.
- Texto com várias fontes do sistema (sans, negrito, serifada, elegante, mono, impacto, condensada, manuscrita) e opção de texto com fundo.
- Desfazer e refazer.

**Editor**
- 12 degradês prontos, degradê personalizado (duas cores e ângulo), cor sólida, fundo com a própria captura borrada ou transparente.
- Margem, cantos arredondados, sombra, barra de janela e formatos 16:9, 4:3, 1:1 e 9:16.
- Marca d'água com logo (PNG, JPG, WebP) e texto: 9 posições (cantos, bordas, centro) ou faixa superior/inferior com cor própria; tamanho, distância e opacidade ajustáveis; sobre o fundo ou sobre a captura.
- Copiar, salvar, salvar como e fixar na tela.

**Saída**
- Sempre PNG sem perdas na resolução original da tela. Nada de envio para nuvem nem busca de imagem.
- Copiar ou salvar direto da captura pode aplicar o estilo completo, só os cantos arredondados ou nada (em Ajustes).

**Extras**
- Fixar uma captura por cima das outras janelas (arrastar move, roda do mouse dá zoom, duplo clique fecha).
- Arrastar uma imagem para a janela principal ou colar com `Ctrl+V` abre no editor.

## Atalhos

| Tecla | Ação |
|---|---|
| `PrtSc` (Mac: `Ctrl+Shift+2`) | Capturar área |
| `Shift+PrtSc` (Mac: `Ctrl+Shift+1`) | Tela inteira |
| `P` `L` `A` `R` `O` `M` `T` `B` `N` | Caneta, linha, seta, retângulo, círculo, marcador, texto, borrar, numeração |
| `V` | Mover e ajustar a seleção |
| `Ctrl+C` / `Ctrl+S` | Copiar / salvar |
| `Enter` ou `Ctrl+E` | Abrir no editor |
| `Ctrl+T` | Fixar na tela |
| `Ctrl+Z` / `Ctrl+Y` | Desfazer / refazer |
| `Esc` | Sair da ferramenta ou fechar |

Os atalhos globais funcionam enquanto a janela principal estiver aberta (pode deixar minimizada).

## Download

Baixe em [Releases](../../releases) ou nos artefatos da última execução do workflow `build`:

- **Windows**: `takeshot-windows-x64.zip`. Se o `PrtSc` abrir a Ferramenta de Captura do Windows, desligue em Configurações > Acessibilidade > Teclado > "Usar a tecla Print Screen para abrir a captura de tela".
- **macOS** (Apple Silicon e Intel): `takeshot-macos-universal.zip`. O app não é assinado pela Apple: na primeira vez, clique com o botão direito em Takeshot.app > Abrir, ou rode `xattr -cr Takeshot.app`. Libere em Ajustes do Sistema > Privacidade e Segurança > Gravação de Tela.
- **Linux**: `takeshot-linux-x64.tar.gz`. Atalhos globais precisam de X11; no Wayland a captura passa pelo portal do sistema.

## Compilar

```sh
cargo build --release
```

No Windows o `rust-toolchain.toml` usa o toolchain MSVC. Em outros sistemas rode com `RUSTUP_TOOLCHAIN=stable cargo build --release`. No Linux instale antes: `libgtk-3-dev libxcb1-dev libxcb-randr0-dev libxkbcommon-dev libwayland-dev libdbus-1-dev libpipewire-0.3-dev libclang-dev libegl1-mesa-dev libgbm-dev`.

`takeshot --screenshot saida.png` abre a janela, desenha 120 quadros e salva a imagem dela (usado na verificação automática). `takeshot --edit` abre o editor com uma imagem de exemplo.

## Licença

Copyright (c) 2026-present Takeflow Sistemas

Licenciado sob a [GNU Affero General Public License v3.0 ou posterior](LICENSE). Avisos de terceiros em [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md).
