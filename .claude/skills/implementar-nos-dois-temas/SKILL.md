---
name: implementar-nos-dois-temas
description: Use ao criar ou mudar qualquer função, tela ou texto visível do Takeshot.
---

# Implementar nos dois temas

Antes de começar, leia a seção Parity do `AGENTS.md`: os dois temas dividem um layout só, e um tema muda cores, marca, nome e o visual de alguns componentes do kit, nunca o lugar de uma função. Referência visual: `design/takeshot-ui.fig`, página "A · Família TakeFlow" para o layout e o visual da TakeUI, e o quadro "B · Design system" da página "B · Estúdio coral" para as cores e os componentes da Coral UI.

## Passos

1. **Núcleo.** Ponha o comportamento em `src/engine/`, `src/system/` ou no estado de `src/screens/`, nunca dentro de uma vista. Escreva antes o teste que falha, veja-o falhar e depois passar. Pronto quando o teste novo passa e `cargo test` inteiro está verde.
2. **Vista.** Desenhe a função em `src/views/`, no lugar indicado pela página A do `.fig`, com os componentes de `src/ui/kit.rs` e as cores de `Tokens`. Uma diferença de visual entre os temas mora no kit, nos tokens ou em `ui::brand`; a vista não pergunta qual é o tema para mudar um lugar. Pronto quando a função funciona no app.
3. **Textos.** Cada texto novo vira uma chave `Msg` em `src/ui/i18n.rs`, com inglês e português. Pronto quando o projeto compila (uma tradução faltando não compila).
4. **Os quatro visuais.** Gere as quatro combinações de `--theme takeui|coral` com `--mode light|dark` da tela mexida, cada uma com `--screenshot <arquivo>.png` (a galeria é `--page componentes`; `scripts/screenshots.sh` faz todas de uma vez), e compare a TakeUI com a página A e a Coral UI com o quadro "B · Design system". Para acompanhar enquanto edita, rode `scripts/dev.sh` com os mesmos argumentos. Pronto quando as quatro imagens estão certas e anexadas ao PR.
5. **PR.** Preencha o template, com as telas, a linha de paridade (os quatro visuais certos ou o que ainda destoa), as traduções e as verificações. Pronto quando cada item marcado tem evidência no PR.
