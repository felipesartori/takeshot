---
name: implementar-nos-dois-temas
description: Use ao criar ou mudar qualquer função, tela ou texto visível do Takeshot.
---

# Implementar nos dois temas

O Takeshot tem dois temas completos, TakeUI e Coral UI, cada um com claro, escuro e sistema. Paridade: toda função visível existe nos dois, cada uma no lugar que o design do seu tema dá a ela. A referência visual é o `design/takeshot-ui.fig`: página "A · Família TakeFlow" (TakeUI), "B · Estúdio coral" (Coral UI) e "Comparar A e B".

## Passos

1. **Núcleo.** Ponha o comportamento no núcleo (estado, objetos, ações), nunca dentro de uma pele. Escreva antes o teste que falha, veja-o falhar e depois passar. Pronto quando o teste novo passa e `cargo test` inteiro está verde.
2. **TakeUI.** Ligue a função na pele TakeUI, no lugar indicado pela página A do `.fig`. Pronto quando a função funciona no app com o tema TakeUI.
3. **Coral UI.** Ligue a mesma função na pele Coral UI, no lugar indicado pela página B. Se o `.fig` não mostra esse lugar, use a página "Comparar A e B" e escolha o lugar equivalente no padrão da Coral UI. Pronto quando a função funciona no app com o tema Coral UI.
4. **Textos.** Cada texto novo entra em inglês e em português na mesma mudança. Pronto quando nenhum texto visível novo existe em uma língua só.
5. **Capturas.** Gere as quatro combinações (TakeUI e Coral UI, claro e escuro) e compare cada uma com o `.fig`. Pronto quando as quatro imagens estão anexadas ao PR.
6. **PR.** Preencha o template, com os checkboxes de telas, paridade, traduções e verificações. Pronto quando cada checkbox marcado tem evidência no PR.

## Enquanto as peles não existem

O redesign que cria as duas peles está em andamento na branch `feat/redesign`. Até ele chegar, mude a interface atual e escreva no PR o que falta para o outro tema; os passos 2, 3 e 5 passam a valer quando as peles existirem.
