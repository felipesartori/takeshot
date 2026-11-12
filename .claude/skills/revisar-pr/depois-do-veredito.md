# Depois do veredito: corrige, publica, mergeia

A revisão termina em merge, sempre, e sem esperar palavra de comando do usuário. Antes do veredito você só lê e executa; depois dele, nesta ordem:

## 1. Corrige, se o veredito é NO-GO

NO-GO (algum achado Critical ou Important) não vira "pedir mudanças": a gente corrige na branch do contribuidor. GO passa direto ao passo 2. Minor entra no relatório e a correção dele é opcional.

- Exige `maintainerCanModify` verdadeiro. Sem ele não há como corrigir na branch dele: publique o review com `--request-changes` e pare, dizendo isso ao usuário.
- Corrija só o que o review achou e pertence ao tema do PR. Problema fora do tema vira issue e PR nossos.
- Teste que falha antes e passa depois (TDD), a mutação do achado morta pelo teste novo, `cargo fmt --check`, clippy e testes verdes, `scripts/check.sh` (é o hook de pre-push) e o cenário ponta a ponta de novo, no exe da branch corrigida (seção 5 da skill).
- O patch sugerido no review passa pelo mesmo desafio de mutação: o do #9 testava só o helper e deixava solto o call site, onde estava o defeito. Tire o guard do call site e confira que um teste que passa pelo código real (`Home::begin`, `Editor::canvas`...) falha, e que o estado do bug original reaparece na mensagem de falha.
- O commit vai em cima do head do autor, não do estado mergeado com a `main` (que é só de teste): Conventional em inglês, sem `Co-Authored-By` nem trailer de sessão. Corrigindo e testando no estado mergeado, faça o commit ali, depois `git switch -c push-<n> <head do autor> && git cherry-pick <commit>` e empurre essa branch (`HEAD:<branch do PR>`); confira com `git diff --stat <head do autor> HEAD`. Um push de mantenedor roda o CI sem "Approve and run".
- Desenho desatualizado (achado de Standards da seção 5, item 4) corrige-se no mesmo commit: atualize os quadros das telas mexidas em `design/takeshot-ui.fig` nos quatro visuais a partir dos renders do exe do PR, e confira cada quadro contra o render (receita no passo 8 de `implementar-issue`). O `.fig` é binário: não vai por merge de texto, então o commit parte do `.fig` do head do autor.
- Todo achado que a execução provou e que pertence ao tema do PR entra no commit, com o teste dele (vermelho antes, verde depois). O que o ambiente não alcança (macOS, escalas diferentes) não se corrige às cegas: vira "não verificado" no review, com a evidência de leitura, e o merge segue só se o recurso é opt-in e o autor já o declarou.
- Push sem force pela URL do fork, com o token do gh e sem prompt: `GIT_TERMINAL_PROMPT=0 git -c credential.helper= -c "credential.helper=!gh auth git-credential" push https://github.com/<dono do fork>/<repo>.git HEAD:<branch do PR>`. O `.githooks/pre-push` roda o `scripts/check.sh` dentro do push (de 1 a 5 min): use `timeout` longo ou background, e não mate o push no meio do hook (foi o que pareceu um prompt de credencial no #9).

## 2. Publica

`gh pr review <n> --repo takeflow-oficial/takeshot --approve --body-file shots/review-pr-<n>.md` (inglês, sem travessão). O texto traz o veredito, os achados, o que a gente ajustou (com o hash do commit) e o que ficou de fora.

## 3. Mergeia

Merge commit, nunca squash: `gh pr merge <n> --repo takeflow-oficial/takeshot --merge --match-head-commit <sha do head atual>`, depois de `gh pr checks <n>` verde lido direto, no head novo, e não pelo cache de um cliente. Em pilha, na ordem: o commit do contribuidor mantém o hash e os seguintes não precisam de rebase.

## Quando parar e avisar o usuário

Sem merge, só relatório: gate ou ponta a ponta que falha e a correção não fecha; correção que sairia do tema do PR; CI vermelho; `maintainerCanModify` falso. Nunca: force-push, squash, commit na `main`, merge sem CI verde no head que vai entrar.

## Fechamento

1. Uma linha por PR com o veredito em `.opencode/memory/state.md`, e as pegadinhas novas em `gotchas.md` (ambos ficam fora do git).
2. Devolva o checkout: `git switch <branch de origem>`, `git branch -D pr-<n>` (e a branch temporária do estado mergeado, se criou) e `git stash pop` se você guardou trabalho na seção 3.
3. Mantenha exes e roteiro em `shots/pr-<n>`. `target-review` só ao fim da pilha.
