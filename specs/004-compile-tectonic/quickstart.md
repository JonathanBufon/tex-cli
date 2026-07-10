# Phase 1 — Quickstart: Compilação `.tex` → PDF

**Feature**: `004-compile-tectonic`
**Date**: 2026-07-10

Guia dev pra buildar, testar e exercitar o novo subcomando
`compile`. Pré-requisito duro: specs 001–003 mergeadas em `main`.

---

## 1. Pré-requisitos

### Host

- Rust stable via `rustup`.
- Docker (para os testes de integração isolados).
- `git`.

### Docker

Container `tex-cli` das specs anteriores. **Já tem tectonic
instalado** (spec 001, Dockerfile). Nenhuma mudança nesta spec.

Para testar `--engine latexmk`/`pdflatex`/etc. na CI, uma spec
futura pode adicionar esses binários ao Dockerfile. Nesta v1 só
tectonic é testado end-to-end; os outros engines usam pattern
padrão (documentado no research D-03) e são verificados apenas via
teste de "engine ausente" (`PATH=/tmp/empty`).

---

## 2. Build local

Nenhuma nova dep — o Cargo.toml permanece intocado por esta
feature. Compile incremental subsequente < 5 s.

```bash
git checkout 004-compile-tectonic-impl
docker run --rm -v $(pwd):/src -w /src tex-cli cargo build
```

---

## 3. Setup mínimo

Assume `tex-cli` já instalado localmente. Reusa init (spec 001),
templates add (spec 002), render (spec 003):

```bash
mkdir -p ~/tex-tmp/{templates,output}

tex-cli init \
  --templates-dir ~/tex-tmp/templates \
  --output-dir    ~/tex-tmp/output \
  --engine        tectonic --force

# Adiciona template + JSON e renderiza:
tex-cli templates add examples/templates/artigo-basico.tex
tex-cli render artigo-basico examples/data/artigo-basico.json
# → ~/tex-tmp/output/artigo-basico.tex existe
```

---

## 4. Rodar `compile` manualmente

### Compilação básica

```bash
tex-cli compile ~/tex-tmp/output/artigo-basico.tex
# → stdout: PDF gerado em /home/user/tex-tmp/output/artigo-basico.pdf.
#           Compilação levou 1.4s.

# Verificar magic bytes:
head -c 4 ~/tex-tmp/output/artigo-basico.pdf
# → %PDF
```

### Output custom

```bash
tex-cli compile ~/tex-tmp/output/artigo-basico.tex \
  --output /tmp/final.pdf
```

### Engine override sem alterar config

```bash
# Config diz tectonic:
tex-cli config show --format json | jq .compiler.engine
# → "tectonic"

# Compila com latexmk (assumindo instalado):
tex-cli compile ~/tex-tmp/output/artigo-basico.tex --engine latexmk

# Config continua tectonic:
tex-cli config show --format json | jq .compiler.engine
# → "tectonic"
```

### Manter `.tex` e `.log`

```bash
tex-cli compile ~/tex-tmp/output/artigo-basico.tex --keep-tex --keep-logs
ls -la ~/tex-tmp/output/artigo-basico.*
# → artigo-basico.pdf
# → artigo-basico.tex
# → artigo-basico.log
```

### Sobrescrever com --force

```bash
tex-cli compile ~/tex-tmp/output/artigo-basico.tex             # 1ª vez
tex-cli compile ~/tex-tmp/output/artigo-basico.tex --force     # sobrescreve
# → PDF gerado (sobrescrito) em .../artigo-basico.pdf. Compilação levou 1.3s.
```

### Modo interativo

```bash
tex-cli compile
# ? Caminho do .tex a compilar: › /home/user/tex-tmp/output/artigo-basico.tex
# ? Manter cópia do .tex no output_dir? [Y/n] › Y
# ? Manter cópia do .log no output_dir? [Y/n] › Y
# → PDF gerado em .../artigo-basico.pdf.
```

---

## 5. Falhas esperadas (validam contratos)

### `.tex` inexistente

```bash
tex-cli compile /tmp/nao-existe.tex
# stderr: arquivo /tmp/nao-existe.tex não encontrado.
# exit 1
```

### `.tex` com erro de sintaxe LaTeX

```bash
cat > /tmp/broken.tex <<'EOF'
\documentclass{article}
\begin{document}
\undefinedcommand{oops}
% missing \end{document}
EOF

tex-cli compile /tmp/broken.tex --keep-logs
# stderr: Falha ao compilar '/tmp/broken.tex': engine 'tectonic' retornou erro.
# Últimas linhas do log:
# ! Undefined control sequence.
# l.3 \undefinedcommand{oops}
# ...
# exit 40
```

### Engine ausente

```bash
PATH=/tmp/empty tex-cli compile ~/tex-tmp/output/artigo-basico.tex
# stderr: Engine 'tectonic' não está instalado no PATH.
#         Instale-o antes ou use --engine <outro>.
# exit 41
```

### Engine não suportado

```bash
tex-cli compile ~/tex-tmp/output/artigo-basico.tex --engine foo
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
tex-cli compile ~/tex-tmp/output/artigo-basico.tex --keep-tex --no-keep-tex
# stderr: error: the argument '--keep-tex' cannot be used with '--no-keep-tex'
# exit 2
```

### Sobrescrever PDF sem --force em não-TTY

```bash
tex-cli compile ~/tex-tmp/output/artigo-basico.tex               # 1ª vez OK
tex-cli compile ~/tex-tmp/output/artigo-basico.tex < /dev/null   # sem TTY
# stderr: Arquivo <...>/artigo-basico.pdf já existe. Use --force
#         ou execute em terminal interativo.
# exit 15
```

---

## 6. Testes

Suite completa (specs 001+002+003+004):

```bash
docker run --rm --name tex-cli \
  -v "$(pwd):/src" -w /src \
  tex-cli cargo test --all
```

Só testes desta spec:

```bash
docker run --rm -v "$(pwd):/src" -w /src tex-cli \
  cargo test --test cli_compile
```

Unit tests do módulo compiler:

```bash
docker run --rm -v "$(pwd):/src" -w /src tex-cli \
  cargo test --lib compiler
```

Higiene (fmt + clippy) — mesmo comando das specs anteriores.

---

## 7. Baseline de performance esperada

Alvos do plan:

| Métrica                                          | Alvo    | Notas                              |
|--------------------------------------------------|---------|------------------------------------|
| `compile` template simples (<5 pág.) via tectonic| <15 s   | SC-001 (cold-start incluído)       |
| Ciclo `cargo test --all` incremental             | <15 s   | Alguns testes reais compilam .tex  |
| `cargo test --lib compiler` (só unit)            | <1 s    | Puramente in-memory                |

Comando de smoke bench:

```bash
for run in 1 2 3; do
  START=$(date +%s%N)
  ./target/release/tex-cli compile ~/tex-tmp/output/artigo-basico.tex --force > /dev/null
  END=$(date +%s%N)
  echo "run $run: $(( (END - START) / 1000000 )) ms"
done
```

---

## 8. Troubleshooting

| Sintoma                                              | Causa provável                              | Ação                                           |
|------------------------------------------------------|---------------------------------------------|------------------------------------------------|
| `arquivo <path> não encontrado` (exit 1)             | Path do `.tex` errado                       | Verifique com `ls`, use path absoluto.         |
| `Falha ao compilar '…': engine '…' retornou erro` (exit 40) | Erro de sintaxe LaTeX                | Leia o tail do log; se cortou, rode com `--keep-logs` e abra o arquivo completo. |
| `Engine '…' não está instalado no PATH` (exit 41)    | Engine ausente                              | Instale via distro package manager ou `install-tectonic.sh`. |
| `Engine '…' não é suportado` (exit 42)               | Nome fora dos 5 canônicos                   | Use exatamente `tectonic`, `latexmk`, `pdflatex`, `xelatex` ou `lualatex`. |
| Flags conflitantes (exit 2)                          | `--keep-tex` + `--no-keep-tex`              | Use apenas uma das duas.                       |
| `Arquivo … já existe` (exit 15)                      | Overwrite sem `--force` em não-TTY          | Adicione `--force` ou rode em TTY.             |
| PDF gerado mas incompleto/vazio                      | Compilação exit=0 sem produzir PDF válido   | Rode com `-vv` pra ver o stdout do engine em tempo real, ou `--keep-logs`. |
| Compilação demora muito                              | Documento grande + várias passes de bib     | Considere trocar pra `--engine latexmk` (múltiplas passes automáticas). |
