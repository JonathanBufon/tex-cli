# Phase 1 — CLI Contract: `tex-cli build`

**Feature**: `005-build-pipeline`
**Date**: 2026-07-10

Contrato observável do subcomando `build` + menu interativo.
Herdado das specs 001–004 (não repetido): banner em stderr,
`-v/--verbose`, `--help/--version`, exit codes 0/1/2/10/11/14/15/20/22/30/31/40/41/42.

---

## `tex-cli build <template> <data-source> [--output <path>] [--engine <name>] [--keep-tex|--no-keep-tex] [--keep-logs|--no-keep-logs] [--force]`

### Grammar

```text
tex-cli build <template> <data-source>
tex-cli build artigo dados.json
tex-cli build artigo dados.json --output /tmp/final.pdf
tex-cli build artigo dados.json --engine latexmk
tex-cli build artigo dados.json --keep-tex --keep-logs
tex-cli build artigo dados.json --no-keep-tex
tex-cli build artigo dados.json --force
tex-cli build artigo -                      # JSON via stdin
tex-cli build artigo dados.json --engine tectonic --output /tmp/x.pdf --keep-tex --force
```

Args positionais **obrigatórios** (ambos), a menos que ambos sejam
omitidos (→ menu interativo):

- `<template>`: nome do template (com ou sem `.tex`), resolvido
  em `config.paths.templates_dir` via `resolve_template` (spec 002).
- `<data-source>`: path do JSON no host OU literal `-` para stdin
  (mesmo pattern do `render` spec 003).

Flags:

| Flag                  | Curta | Efeito                                                       |
|-----------------------|-------|--------------------------------------------------------------|
| `--output <path>`     | `-o`  | Path custom do PDF final. Aceita `~`/relativos.              |
| `--engine <name>`     | `-e`  | Sobrescreve `compiler.engine` só nesta invocação.            |
| `--keep-tex`          | —     | Copia `.tex` intermediário para `output_dir` ANTES do compile. |
| `--no-keep-tex`       | —     | Descarta `.tex` intermediário mesmo se config diz true.      |
| `--keep-logs`         | —     | Copia `.log` do engine para `output_dir` após compile.       |
| `--no-keep-logs`      | —     | Descarta `.log`.                                             |
| `--force`             | —     | Sobrescreve PDF existente sem prompt.                         |

Flags conflitantes (`--keep-tex --no-keep-tex` OU `--keep-logs --no-keep-logs`)
→ exit `2` (clap enforce, mesmo pattern do `compile`).

### Behavior

1. Carrega config (10/11).
2. Resolve `template_name` e `data_source` de args positionais.
   Se ambos ausentes → delega para `handle_build_menu(&cfg)`.
3. Resolve `engine`:
   - `--engine <name>` → parse via `SupportedEngine::from_str`;
     falha → exit `42`.
   - Ausente → mesmo parse com `config.compiler.engine`.
4. Resolve `output_pdf`:
   - `--output` → `expand_user_path`.
   - Ausente → `config.paths.output_dir/<template>.pdf`.
5. Resolve `keep_tex`/`keep_logs`: flag → config default.
6. Overwrite guard: se `output_pdf.exists()` e `!--force`:
   - TTY → `inquire::Confirm` "Sobrescrever `<path>`?" default "não";
     "não" → exit `15`.
   - Não-TTY → exit `15` (mesmo padrão do compile).
7. Chama `build_pipeline(cfg, template, data_source, output_pdf,
   engine, keep_tex, keep_logs, force, verbose)`:
   - Etapa **render** (spec 003): read_template → load_json_source
     → render_template. Erros: 20/22/30/31/14/1.
   - Etapa **write intermediate** (se `keep_tex=true`): grava
     `<output_dir>/<template>.tex` atomicamente (FR-015 — vem
     antes do compile).
   - Etapa **compile** (spec 004): TempDir + write `.tex` + invocar
     engine + copiar PDF. Erros: 40/41/14/1. (Nota: `keep_tex`
     passado como `false` para o compile pois já lidamos manualmente.)
8. Se qualquer etapa falhar, exit code é o da etapa. Nenhum PDF
   parcial no destino (compile é atômico). `.tex` intermediário
   permanece **só se** `keep_tex=true` (FR-15).
9. Se sucesso: stdout
   `PDF gerado em <path-absoluto>. Pipeline (render + compile) levou X.Ys.`
   ou `PDF gerado (sobrescrito) em <path>. Pipeline (render +
   compile) levou X.Ys.` (uma casa decimal).
10. Exit `0`.

### Exit codes

Reusa 100% das specs anteriores — **zero código novo**:

- `0` sucesso.
- `1` erro genérico (I/O sem categoria).
- `2` erro de arg CLI (flags conflitantes).
- `10` config ausente.
- `11` config corrompido.
- `14` permissão negada.
- `15` user-aborted.
- `20` template não encontrado.
- `22` templates_dir inexistente.
- `30` tera error (render).
- `31` JSON inválido.
- `40` compile failed (engine retornou erro).
- `41` engine não instalado.
- `42` engine não suportado.

### Saída

- **stdout**: exclusivamente a linha `PDF gerado em … Pipeline
  (render + compile) levou X.Ys.` (sucesso) OU nada (falha — tudo
  em stderr).
- **stderr**: banner + prompts (só TTY + overwrite) + logs `-v`
  granulares por etapa + mensagens de erro com contexto de etapa
  (SC-007).

### Contratos observáveis (testes de integração)

- `build <tpl> <data>` bem-sucedido produz arquivo cujo primeiro
  argumento de `std::fs::read` retorna bytes iniciando com `%PDF-`.
- Modo do PDF gravado = `0o644`.
- `--engine <novo>` **não altera** `.compiler.engine` no config
  file (verificável via `config show --format json | jq
  .compiler.engine` antes e depois — SC-005 herdado).
- Template ausente → exit `20`. Template inválido → não chega ao
  compile (falha antes).
- JSON top-level array → exit `31`. JSON malformado → exit `31`.
- Template com var ausente no JSON → exit `30`.
- Template renderiza OK mas compile falha → exit `40`, stderr
  contém tail do log LaTeX; PDF pré-existente permanece intacto se
  `!--force`. **Se `--keep-tex=true`, `.tex` intermediário
  permanece em `output_dir` para debug** (FR-15).
- Engine ausente do PATH → exit `41`.
- Engine `foo` → exit `42` com lista dos 5 aceitos.
- Zero artefato do TempDir do build ou do compile após comando
  (SC-006).
- Stdin: `echo '{"x":1}' | build tpl -` funciona.
- `--force` sobrescreve PDF existente sem prompt.

---

## `tex-cli build` (sem args, modo interativo)

### Grammar

```text
tex-cli build
```

### Behavior

1. Carrega config (10/11).
2. Guard TTY via `IsTerminal`. Sem TTY → exit `1` com stderr
   `Menu interativo de build requer terminal. Use tex-cli build
   <template> <data.json>.`.
3. Lista templates via `list_templates(&cfg.paths.templates_dir)`.
   Vazio → exit `22` com stderr orientando `tex-cli templates add`.
4. Prompts sequenciais (research D-08):
   - `Select` template (`prompt_template_name(&names)`).
   - `Text` JSON source (`prompt_json_source()`).
   - `Confirm` keep_tex (default = `cfg.compiler.keep_tex`).
   - `Confirm` keep_logs (default = `cfg.compiler.keep_logs`).
5. Constrói `BuildArgs { template_name: Some(...), data_source:
   Some(...), output: None, engine: None, keep_tex, no_keep_tex:
   !keep_tex, keep_logs, no_keep_logs: !keep_logs, force: false }`
   e chama `handle_build(args)` (single choke point).
6. Exit code = exit do `handle_build` resultante.
7. Ctrl+C em qualquer prompt → exit `15`.

### Exit codes

Mesmos do subcomando direto + `1` para "menu requer TTY".

### Contratos observáveis

- Sem TTY: exit 1 sem prompt.
- TTY com templates: sequência de prompts → PDF gerado.
- Templates vazios: exit 22 com mensagem apontando pra `templates
  add`.

---

## Ausências deliberadas (fora do escopo v1)

- Sem `--dry-run` — usa `render --dry-run` para inspecionar `.tex`
  antes de compilar.
- Sem `--timeout <sec>`.
- Sem `--watch`.
- Sem batch (`--all-templates`).
- Sem passthrough de flags do engine.
- Sem output JSON estruturado (`--format json`).
- Sem `--include-dir` para bibs/imagens (mesma limitação da spec 004).
