# PR #1 — Escopo

**Título proposto**: `docs: spec 001 config-init-management (design phase)`

**Branch**: `001-config-init-management` → `main`

**Data**: 2026-07-09

---

## Recomendação de escopo

**PR #1 entrega apenas os documentos de design da spec 001** — nenhum
código Rust é adicionado. A implementação (34 tasks já quebradas em
`tasks.md`) segue em PRs incrementais depois deste.

Racional:

- Cria um **checkpoint de review** claro do design antes de qualquer
  código ser escrito. Se algo estiver errado, é barato ajustar.
- Estabelece a constituição e a spec como **base congelada** —
  qualquer PR de implementação já parte de um contrato fechado.
- Deixa PRs subsequentes pequenos e focados (uma user story cada),
  o que é fácil de revisar.
- É consistente com o fluxo speckit: `specify → clarify → plan →
  tasks` termina aqui; `implement` começa no próximo PR.

---

## O que este PR entrega

### Commits incluídos (4)

```text
84030ae docs(tasks): 001 config-init-management task breakdown
0c45195 docs(plan): 001 config-init-management design phase + TEX CLI banner
8912040 docs(spec): clarify 001 config-init-management
b200ec1 docs(spec): add 001 config-init-management specification
```

(Os commits `chore: bootstrap speckit tooling` e `docs: ratify Tex
constitution v1.0.0` **já estão no `main`** — não fazem parte deste PR.)

### Arquivos criados

| Caminho                                                                | Propósito                                                                 |
|------------------------------------------------------------------------|---------------------------------------------------------------------------|
| `specs/001-config-init-management/spec.md`                             | Especificação: user stories P1/P2/P3, 25 FRs, 6 SCs, edge cases, assumptions, clarifications |
| `specs/001-config-init-management/checklists/requirements.md`          | Checklist de qualidade (todos itens ✅)                                    |
| `specs/001-config-init-management/plan.md`                             | Plan: Technical Context, Constitution Check (5/5 PASS), estrutura         |
| `specs/001-config-init-management/research.md`                         | 10 decisões técnicas (D-01..D-10) com rationale e alternativas rejeitadas |
| `specs/001-config-init-management/data-model.md`                       | Struct `Config`, enum `ConfigKey`, `SupportedEngine`, defaults, validação |
| `specs/001-config-init-management/contracts/cli.md`                    | Contrato observável dos 3 subcomandos + banner transversal + exit codes   |
| `specs/001-config-init-management/quickstart.md`                       | Guia dev (build local + Docker + troubleshooting)                         |
| `specs/001-config-init-management/tasks.md`                            | 34 tasks em 6 fases (setup, foundational, US1 MVP, US2, US3, polish)      |
| `specs/001-config-init-management/pr-scope.md`                         | **Este arquivo**                                                          |
| `assets/banner.txt`                                                    | ASCII "TEX CLI" (block letters Unicode, sem bordas `//`)                  |
| `.specify/feature.json`                                                | Ponteiro do speckit para a feature ativa                                  |

### Arquivos alterados

| Caminho       | Mudança                                                                             |
|---------------|-------------------------------------------------------------------------------------|
| `CLAUDE.md`   | Marker SPECKIT atualizado para apontar `specs/001-config-init-management/plan.md`   |

### O que **não** está neste PR

- Nenhum arquivo `Cargo.toml` ou `src/**`.
- Nenhum teste executável.
- Nenhum `docker/Dockerfile`.
- Nenhuma alteração na `constitution.md` (ela veio em PR anterior já
  mergeado).

---

## Alinhamento com a constitution

Constitution Check re-executado após Phase 1 do plan:

| Princípio                                              | Status |
|--------------------------------------------------------|--------|
| I. Escopo pessoal e não-substituição do LaTeX         | ✅ PASS |
| II. Separação Dados/Template/PDF                       | ✅ PASS |
| III. Interface Dupla CLI + Interativo                  | ✅ PASS |
| IV. Compilador Plugável                                | ✅ PASS |
| V. Segurança na Manipulação de Arquivos                | ✅ PASS |

Complexity Tracking do plan permanece vazia — sem violações a justificar.

---

## Test plan (para o reviewer)

Como este PR não tem código, os "testes" aqui são de **revisão
documental**. Sugestão de checklist:

- [ ] Ler `spec.md` — user stories P1/P2/P3 fazem sentido e são
      independentemente testáveis?
- [ ] Ler `plan.md` — Technical Context contempla tudo? Constitution
      Check está honesto?
- [ ] Ler `research.md` — as 10 decisões (D-01..D-10) têm alternativas
      rejeitadas plausíveis? Nenhum "porque sim"?
- [ ] Ler `data-model.md` — a struct `Config` e o enum `ConfigKey`
      cobrem todos os FRs de `spec.md`?
- [ ] Ler `contracts/cli.md` — os 3 subcomandos têm exit codes e
      contratos observáveis suficientes para testes automatizados?
- [ ] Ler `tasks.md` — as 34 tasks cobrem 100% dos FRs e SCs? Alguma
      task fora do escopo desta spec (tocar templates/renderer/compiler)?
- [ ] Abrir `assets/banner.txt` num terminal UTF-8 e conferir que renderiza
      como "TEX CLI" em block letters.
- [ ] Confirmar que nenhum arquivo de código foi adicionado
      (`git diff --stat main..HEAD -- src/ Cargo.toml`).

---

## Sequência de PRs planejada

| PR    | Escopo                                                                   | Tasks         | Depende de      |
|-------|--------------------------------------------------------------------------|---------------|-----------------|
| **#1**| Docs de design da spec 001 (**este PR**)                                 | —             | —               |
| #2    | Setup + Foundational (Cargo, Dockerfile, main.rs, errors, paths, cli skel) | T001–T009    | #1 mergeado     |
| #3    | User Story 1 — `tex-cli init` (MVP)                                      | T010–T018     | #2 mergeado     |
| #4    | User Story 2 — `tex-cli config show`                                     | T019–T023     | #3 mergeado     |
| #5    | User Story 3 — `tex-cli config set`                                      | T024–T029     | #4 mergeado     |
| #6    | Polish (banner test, README, fmt/clippy, run quickstart)                 | T030–T034     | #5 mergeado     |

Cada PR de implementação (#2 em diante) roda o container Docker
(`docker run --rm -v $(pwd):/src -w /src tex-cli cargo test --all`)
antes do merge.

---

## Alternativa considerada: PR #1 grande

Bundlar spec-only com o MVP inteiro (US1 completo) num único PR
(≈ 22 tasks: T001–T018). Vantagem: menos overhead de review. Desvantagem:
mistura design com implementação; se algo do design precisar mudar
depois de escrever código, o custo de revisão fica maior.

**Rejeitado** em favor do PR docs-only por preferência de checkpoints
menores. Reversível se, na prática, PRs de spec forem pesados demais
para o valor entregado.

---

## Ações para abrir o PR

Comando sugerido (a executar quando pronto):

```bash
git push -u origin 001-config-init-management

gh pr create \
  --base main \
  --head 001-config-init-management \
  --title "docs: spec 001 config-init-management (design phase)" \
  --body-file specs/001-config-init-management/pr-scope.md
```

Observação: o `--body-file` aponta para este próprio arquivo — a
descrição do PR é literalmente este documento.
