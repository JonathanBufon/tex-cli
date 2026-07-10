# Phase 1 — Quickstart: Pipeline JSON → PDF (`build`)

**Feature**: `005-build-pipeline`
**Date**: 2026-07-10

Guia dev pra buildar, testar e exercitar o novo subcomando `build`.
Pré-requisito duro: specs 001–004 mergeadas em `main`.

---

## 1. Pré-requisitos

### Host

- Rust stable via `rustup`.
- Docker.
- `git`.

### Docker

Container `tex-cli` das specs anteriores (Debian trixie + Tectonic
0.16.9). **Zero mudança** nesta spec.

---

## 2. Build local

Nenhuma nova dep — o Cargo.toml permanece intocado. Compile
incremental subsequente < 5 s.

```bash
git checkout 005-build-pipeline-impl
docker run --rm -v $(pwd):/src -w /src tex-cli cargo build
```

---

## 3. Setup mínimo

Reusa init (spec 001), templates add (spec 002):

```bash
mkdir -p ~/tex-tmp/{templates,output}

tex-cli init \
  --templates-dir ~/tex-tmp/templates \
  --output-dir    ~/tex-tmp/output \
  --engine        tectonic --force

tex-cli templates add examples/templates/artigo-basico.tex
```

Nada mais é necessário — `build` orquestra render + compile
internamente.

---

## 4. Rodar `build` manualmente

### Pipeline completo em um comando

```bash
tex-cli build artigo-basico examples/data/artigo-basico.json
# → stdout: PDF gerado em /home/user/tex-tmp/output/artigo-basico.pdf.
#           Pipeline (render + compile) levou 2.3s.

# Verificar magic bytes:
head -c 4 ~/tex-tmp/output/artigo-basico.pdf
# → %PDF
```

### Output custom

```bash
tex-cli build artigo-basico examples/data/artigo-basico.json \
  --output /tmp/final.pdf
```

### Preservar `.tex` intermediário

```bash
tex-cli build artigo-basico examples/data/artigo-basico.json --keep-tex
ls ~/tex-tmp/output/
# → artigo-basico.pdf
# → artigo-basico.tex  ← intermediário para debug/inspeção
```

### Preservar `.log` do engine

```bash
tex-cli build artigo-basico examples/data/artigo-basico.json --keep-logs
ls ~/tex-tmp/output/
# → artigo-basico.pdf
# → artigo-basico.log  ← útil para diagnosticar warnings
```

### Engine override sem alterar config

```bash
tex-cli config show --format json | jq .compiler.engine
# → "tectonic"

tex-cli build artigo-basico examples/data/artigo-basico.json \
  --engine latexmk

tex-cli config show --format json | jq .compiler.engine
# → "tectonic"  (imutável, SC-005)
```

### JSON via stdin

```bash
echo '{"titulo":"CLI Test","autor":"CI","data":"2026-07-10",...}' \
  | tex-cli build artigo-basico -
```

### Sobrescrever com --force

```bash
tex-cli build artigo-basico examples/data/artigo-basico.json          # 1ª vez
tex-cli build artigo-basico examples/data/artigo-basico.json --force  # sobrescreve
# → PDF gerado (sobrescrito) em .../artigo-basico.pdf. Pipeline (render + compile) levou 1.9s.
```

### Modo interativo

```bash
tex-cli build
# ? Nome do template: › artigo-basico
# ? Caminho do arquivo JSON (- para stdin): › examples/data/artigo-basico.json
# ? Manter cópia do .tex no output_dir? [Y/n] › N
# ? Manter cópia do .log no output_dir? [Y/n] › N
# → PDF gerado em .../artigo-basico.pdf. Pipeline (render + compile) levou 2.1s.
```

---

## 5. Falhas esperadas (validam contratos)

### Template ausente

```bash
tex-cli build nao-existe examples/data/artigo-basico.json
# stderr: Template 'nao-existe' não existe em ~/tex-tmp/templates.
# exit 20
```

### JSON malformado

```bash
echo 'not json' > /tmp/bad.json
tex-cli build artigo-basico /tmp/bad.json
# stderr: JSON inválido em /tmp/bad.json: expected value at line 1 column 1.
# exit 31
```

### Template com variável ausente no JSON

```bash
echo '{}' > /tmp/empty.json
tex-cli build artigo-basico /tmp/empty.json
# stderr: Falha ao renderizar template 'artigo-basico': Variable `titulo` not found...
# exit 30
```

### Template renderiza mas gera `.tex` inválido

```bash
# Assume um template que expande para \undefinedcommand no .tex:
tex-cli build broken-template dados.json --keep-tex
# stderr: Falha ao compilar '<temp>/broken-template.tex': engine 'tectonic' retornou erro.
#         Últimas linhas do log:
#         ! Undefined control sequence.
#         l.42 \undefinedcommand
# exit 40
# MAS: ~/tex-tmp/output/broken-template.tex EXISTE  ← FR-15 preservou pra debug
```

### Engine não instalado

```bash
PATH=/tmp/empty tex-cli build artigo-basico examples/data/artigo-basico.json
# stderr: Engine 'tectonic' não está instalado no PATH.
# exit 41
```

### Engine não suportado

```bash
tex-cli build artigo-basico examples/data/artigo-basico.json --engine foo
# stderr: Engine 'foo' não é suportado. Aceitos:
#   - tectonic
#   - latexmk
#   - pdflatex
#   - xelatex
#   - lualatex
# exit 42
```

### Flags conflitantes

```bash
tex-cli build artigo-basico examples/data/artigo-basico.json --keep-tex --no-keep-tex
# stderr: error: the argument '--keep-tex' cannot be used with '--no-keep-tex'
# exit 2
```

### Sobrescrever sem --force em não-TTY

```bash
tex-cli build artigo-basico examples/data/artigo-basico.json
tex-cli build artigo-basico examples/data/artigo-basico.json < /dev/null
# stderr: Arquivo <...>/artigo-basico.pdf já existe. Use --force ou execute em terminal interativo.
# exit 15
```

---

## 6. Testes

Suite completa (specs 001+002+003+004+005):

```bash
docker run --rm --name tex-cli \
  -v "$(pwd):/src" -w /src \
  tex-cli cargo test --all
```

Só testes desta spec:

```bash
docker run --rm -v "$(pwd):/src" -w /src tex-cli \
  cargo test --test cli_build
```

Unit tests do módulo build:

```bash
docker run --rm -v "$(pwd):/src" -w /src tex-cli \
  cargo test --lib build
```

Higiene — mesmo comando das specs anteriores.

---

## 7. Baseline de performance esperada

Alvos do plan:

| Métrica                                          | Alvo    | Notas                              |
|--------------------------------------------------|---------|------------------------------------|
| `build` template simples (<5 pág.) via tectonic  | <20 s   | SC-001 (render + compile combinados) |
| Economia vs. render+compile isolados             | >30%    | SC-002 (redução de overhead de processo) |
| Ciclo `cargo test --all` incremental             | <2 min  | Alguns testes compilam .tex real   |
| `cargo test --lib build` (só unit)               | <1 s    | Puramente in-memory                |

Comando de smoke bench comparativo:

```bash
# Baseline: render + compile separados
START=$(date +%s%N)
tex-cli render artigo-basico examples/data/artigo-basico.json > /dev/null
tex-cli compile ~/tex-tmp/output/artigo-basico.tex --force > /dev/null
END=$(date +%s%N)
echo "render+compile: $(( (END - START) / 1000000 )) ms"

# Otimizado: build
START=$(date +%s%N)
tex-cli build artigo-basico examples/data/artigo-basico.json --force > /dev/null
END=$(date +%s%N)
echo "build: $(( (END - START) / 1000000 )) ms"

# Espera-se build < 0.7 × (render+compile)
```

---

## 8. Troubleshooting

| Sintoma                                              | Causa provável                              | Ação                                           |
|------------------------------------------------------|---------------------------------------------|------------------------------------------------|
| `Template '…' não existe em …` (exit 20)             | Nome errado ou template deletado            | `tex-cli templates list` para ver disponíveis. |
| `JSON inválido em …` (exit 31)                       | JSON malformado ou não-objeto no topo       | Validar com `jq empty <file>`.                 |
| `Falha ao renderizar template '…'` (exit 30)         | Variável do template ausente no JSON        | Adicionar chave ou usar `\| default(value="")`. |
| `Falha ao compilar '…'` (exit 40)                    | Template gerou `.tex` com erro LaTeX        | Use `--keep-tex` e abrir o arquivo pra debug.  |
| `Engine '…' não está instalado no PATH` (exit 41)    | Engine ausente                              | Instalar via distro ou install-tectonic.sh.    |
| `Engine '…' não é suportado` (exit 42)               | Nome fora dos 5 canônicos                   | Use `tectonic`/`latexmk`/`pdflatex`/`xelatex`/`lualatex`. |
| Flags conflitantes (exit 2)                          | `--keep-*` + `--no-keep-*`                  | Use apenas uma das duas.                       |
| PDF idêntico ao anterior mesmo mudando JSON          | Config keep_tex=false + build usando .tex antigo? | Só reusa .tex se ambos flag e config concordarem. Verificar com `--keep-tex` para ver o intermediate. |
| Compile timeout muito longo                          | Documento grande + múltiplas passes bib     | Use `--engine latexmk` (multipass automático). |
