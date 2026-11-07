---
name: implementar-nos-dois-temas
description: Use ao criar ou mudar qualquer função, tela ou texto visível do Takeshot.
---

# Implementar nos dois temas

Antes de começar, leia a seção Parity do `AGENTS.md` (o que é paridade e o estado do redesign). Referência visual: `design/takeshot-ui.fig`, páginas "A · Família TakeFlow" (TakeUI), "B · Estúdio coral" (Coral UI) e "Comparar A e B".

## Passos

Enquanto `src/skins/` não existe, as telas atuais ainda não seguem o tema: os passos 2, 3 e 5 valem só para a galeria de componentes. Para uma tela atual, mude a interface de `src/screens/`, capture com os comandos do passo 4 da seção "Verify" do `AGENTS.md` e liste no PR o que cada tema ainda precisa.

1. **Núcleo.** Ponha o comportamento em `src/engine/` ou `src/system/`, nunca dentro de uma pele. Escreva antes o teste que falha, veja-o falhar e depois passar. Pronto quando o teste novo passa e `cargo test` inteiro está verde.
2. **TakeUI.** Ligue a função na pele TakeUI, no lugar indicado pela página A do `.fig`, usando os componentes de `src/ui/kit.rs`. Pronto quando a função funciona no app com `--theme takeui`.
3. **Coral UI.** Ligue a mesma função na pele Coral UI, no lugar indicado pela página B. Se o `.fig` não mostra esse lugar, use a página "Comparar A e B" e escolha o lugar equivalente no padrão da Coral UI. Pronto quando a função funciona no app com `--theme coral`.
4. **Textos.** Cada texto novo vira uma chave `Msg` em `src/ui/i18n.rs`, com inglês e português. Pronto quando o projeto compila (uma tradução faltando não compila).
5. **Capturas.** Gere as quatro combinações de `--theme takeui|coral` com `--mode light|dark` da tela mexida, cada uma com `--screenshot <arquivo>.png` (a galeria é `--page componentes`; `scripts/screenshots.sh` faz as quatro de uma vez), e compare cada uma com o `.fig`. Para acompanhar enquanto edita, rode `scripts/dev.sh` com os mesmos argumentos. Pronto quando as quatro imagens estão anexadas ao PR.
6. **PR.** Preencha o template, com as telas, o que falta no outro tema, as traduções e as verificações. Pronto quando cada item marcado tem evidência no PR.
