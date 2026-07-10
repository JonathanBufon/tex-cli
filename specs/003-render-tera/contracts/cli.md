# Phase 1 — CLI Contract: `tex-cli render`

**Feature**: `003-render-tera`
**Date**: 2026-07-09

Contrato observável do subcomando `render` + modo interativo.
Herdado das specs 001/002 (não repetido): banner em stderr,
`-v/--verbose`, `--help/--version`, exit codes 0/1/2/10/11/14/15/20/22.

---

## `tex-cli render <template> <data-source> [--output <path>] [--dry-run] [--force]`

### Grammar

```text
tex-cli render <template-name> <data-source>
tex-cli render artigo dados.json
tex-cli render artigo dados.json --output /tmp/out.tex
tex-cli render artigo dados.json --dry-run
tex-cli render artigo dados.json --force
tex-cli render artigo dados.json --output ~/x.tex --force
tex-cli render artigo -                        # JSON via stdin
```

Args positionais **obrigatórios** (ambos):

- `<template-name>`: nome do template (com ou sem `.tex`),
  resolvido em `config.paths.templates_dir` via `resolve_template`
  (spec 002).
- `<data-source>`: path do arquivo JSON no host OU literal `-` para
  stdin.

Flags:

| Flag              | Curta | Efeito                                                   |
|-------------------|-------|----------------------------------------------------------|
| `--output <path>` | `-o`  | Path custom do `.tex` final. Aceita `~`/relativos.       |
| `--dry-run`       | —     | Imprime em stdout, não grava. Ignora `--output` (com warn em `-v`). |
| `--force`         | —     | Sobrescreve destino existente sem prompt.                |

### Behavior

1. Carrega config via `Config::load(config_file_path()?)?`.
   - Ausente → exit `10`, stderr `Nenhum config encontrado. Rode
     'tex-cli init' primeiro.`.
   - Corrompido → exit `11`.
2. Resolve `template_src` via `read_template(&cfg.paths.templates_dir,
   &template_name)?` (spec 002).
   - Template não existe → exit `20`.
   - `templates_dir` não existe → exit `22`.
3. Lê JSON:
   - `data_source == "-"` → `io::stdin()` para `String`.
   - Senão → `fs::read_to_string(data_source)`.
   - Parse via `serde_json::from_str::<Value>` → exit `31` em
     falha com stderr `JSON inválido em <source>: <detail>.`.
   - Verifica `value.is_object()` → exit `31` se não, stderr
     `JSON inválido em <source>: esperado objeto no topo, recebido
     <tipo>.`.
4. Renderiza via `render_template(template_name, template_src, &value)`:
   - Erros do tera (var indefinida, syntax, filtro desconhecido,
     tipo incompatível) → exit `30`, stderr `Falha ao renderizar
     template '<name>': <mensagem tera (inclui line:col em >90% dos
     casos)>.`.
5. Se `--dry-run`:
   - `io::stdout().write_all(rendered.as_bytes())`.
   - stderr **não** ganha nenhum sumário (dry-run é silencioso —
     mensagem final iria contaminar a saída pipeada).
   - Exit `0`.
6. Se **não** dry-run:
   - `output_path = --output` (expandido) ou
     `cfg.paths.output_dir/<template-name>.tex`.
   - `output_path.exists()` + `!force`:
     - TTY → `inquire::Confirm` "Sobrescrever `<path>`?" default
       "não"; resposta "não" → exit `15`.
     - Não-TTY → exit `15`, stderr `Arquivo <path> já existe. Use
       --force ou execute em terminal interativo.`.
   - `atomic::write_atomic(&output_path, rendered.as_bytes(),
     0o644)?`. Cria dirs pais via `create_dir_all` (D-03/D-10).
   - stdout: `Renderizado em <path-absoluto>.` ou `Renderizado
     (sobrescrito) em <path>.`
   - Exit `0`.

### Exit codes

- `0` sucesso.
- `1` erro genérico (`anyhow`, incluindo I/O sem categoria).
- `2` erro de argumento CLI (via clap: template ou data-source
  ausente).
- `10` config ausente.
- `11` config corrompido.
- `14` permissão negada.
- `15` user-aborted (dry-run não emite; só o caminho normal).
- `20` template não encontrado.
- `22` `templates_dir` inexistente.
- `30` erro do tera durante render.
- `31` JSON inválido (parse ou forma).

### Saída

- **stdout**:
  - `--dry-run`: **exclusivamente** o `.tex` renderizado, byte-a-byte.
  - normal: uma única linha de confirmação (`Renderizado ...`).
- **stderr**: banner + prompts (só quando TTY e overwrite) + logs
  de verbose + mensagens de erro.

### Contratos observáveis (testes de integração)

- `render tpl data.json` grava `output_dir/tpl.tex` com conteúdo
  renderizado, modo `0o644`, mtime atualizado.
- `render tpl data.json --dry-run > /tmp/x.tex` produz arquivo
  byte-a-byte idêntico ao que seria gravado sem `--dry-run` (SC-005).
- `render tpl data.json --dry-run | wc -c` retorna o tamanho do
  `.tex` renderizado, sem contar banner ou prefixo.
- `render tpl bad.json` (JSON malformado) → exit `31`, nenhum
  arquivo criado.
- `render tpl data.json` com template que usa variável ausente no
  JSON → exit `30`, stderr menciona linha:col, nenhum arquivo criado.
- `render tpl data.json --output /tmp/x.tex --force` sobrescreve
  `/tmp/x.tex` sem prompt.
- `echo '{"x":1}' | render tpl -` funciona (SC-002 via stdin).
- `render tpl data.json --dry-run --output /tmp/x.tex` produz
  stdout com `.tex` E NÃO cria `/tmp/x.tex`; com `-v`, stderr
  contém `warn: --output ignorado (dry-run)`.

---

## `tex-cli render` (sem args, modo interativo)

### Grammar

```text
tex-cli render
```

### Behavior

1. `Config::load` (exit 10/11).
2. Guard TTY via `std::io::IsTerminal`:
   - Não-TTY → exit `1` com stderr `Menu interativo de render
     requer terminal. Use tex-cli render <template> <data.json>.`.
3. Carrega lista de templates via `list_templates(&cfg.paths.templates_dir)`
   (spec 002). Vazio → exit `22` (herdando semântica: dir vazio no
   contexto de render onde não há o que renderizar) com stderr
   `Nenhum template encontrado em <path>. Adicione um com
   tex-cli templates add.`.
4. Prompts sequenciais:
   - `inquire::Select` populado por `prompt_template_name(&nomes)`
     (spec 002 helper).
   - `inquire::Text` para "Caminho do arquivo JSON (`-` para
     stdin):".
   - `inquire::Confirm` "Modo dry-run? [y/N]" default "não".
5. Constrói `RenderArgs { template_name, data_source, output: None,
   dry_run: <confirm>, force: false }` e chama `handle_render(args)`.
6. Exit code = exit do `handle_render` resultante.
7. Ctrl+C em qualquer prompt → exit `15`.

### Exit codes

Mesmos do subcomando direto + `1` para "menu requer TTY" e `22`
para "diretório vazio".

### Contratos observáveis

- Sem TTY: retorna `1` sem tentar prompt.
- TTY sem templates: retorna `22` com mensagem apontando pra
  `templates add`.
- TTY com templates: escolher X + `dados.json` + "não" pro dry-run
  produz saída equivalente a `render X dados.json`.

---

## Ausências deliberadas (fora do escopo v1)

- Nenhum filtro custom no tera (ex.: `latex_escape`) — usuário escapa
  no template. Enhancement futuro possível.
- Nenhum `--strict` / `--relaxed` — tera default (variável ausente é
  erro) é o único modo desta v1.
- Nenhum `--json-value <json-inline>` — dado sempre vem de arquivo
  ou stdin.
- Nenhum batch (`--all-templates`, glob de JSONs) — um render por
  invocação.
- Nenhum `--output-format=<tex|txt>` — sempre `.tex`.
- Nenhuma resolução automática de `\input{...}` do LaTeX — texto
  literal, compilador que resolve (spec 004).
