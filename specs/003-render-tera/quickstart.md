# Phase 1 — Quickstart: Renderização JSON → `.tex` via Tera

**Feature**: `003-render-tera`
**Date**: 2026-07-09

Guia dev pra buildar, testar e exercitar o novo subcomando `render`.
Pré-requisito duro: specs 001 e 002 mergeadas em `main`.

---

## 1. Pré-requisitos

### Host

- Rust stable via `rustup`.
- Docker.
- `git`.

### Docker

Container `tex-cli` das specs anteriores. Nenhuma mudança nesta
spec.

---

## 2. Build local

**Uma nova dep é adicionada**: `tera` ao `Cargo.toml`. Após checkout:

```bash
git checkout 003-render-tera-impl
docker run --rm -v $(pwd):/src -w /src tex-cli cargo build
```

Primeira compilação leva ~1-2 min extras baixando/compilando o
`tera` e sua stack (`pest`, etc.). Compilação incremental subsequente
< 5 s.

---

## 3. Setup mínimo pra rodar

Assume `tex-cli` já instalado localmente ou via `cargo run`. Reusa
a infra da spec 001 (init) e 002 (templates add).

```bash
mkdir -p ~/tex-tmp/templates ~/tex-tmp/output

tex-cli init \
  --templates-dir ~/tex-tmp/templates \
  --output-dir    ~/tex-tmp/output \
  --engine        tectonic --force

# Adiciona um template com um placeholder simples:
cat > /tmp/greeting.tex <<'EOF'
\documentclass{article}
\begin{document}
Olá, {{ nome }}. Bem-vindo ao {{ evento | default(value="evento") }}.
\end{document}
EOF
tex-cli templates add /tmp/greeting.tex

# JSON de dados:
cat > /tmp/greeting.json <<'EOF'
{
  "nome": "Jonathan",
  "evento": "TEX CLI Sprint"
}
EOF
```

---

## 4. Rodar `render` manualmente

### Renderização básica

```bash
tex-cli render greeting /tmp/greeting.json
# → stdout: Renderizado em /home/user/tex-tmp/output/greeting.tex.

cat ~/tex-tmp/output/greeting.tex
# → \documentclass{article}
#   \begin{document}
#   Olá, Jonathan. Bem-vindo ao TEX CLI Sprint.
#   \end{document}
```

### Output custom

```bash
tex-cli render greeting /tmp/greeting.json --output /tmp/out.tex
# → Renderizado em /tmp/out.tex.
```

### Dry-run (stdout, sem gravar)

```bash
tex-cli render greeting /tmp/greeting.json --dry-run
# → stdout tem o .tex renderizado, nada gravado no output_dir.
```

### JSON via stdin

```bash
echo '{"nome":"Ana","evento":"WWDC"}' | tex-cli render greeting -
# → Renderizado em ~/tex-tmp/output/greeting.tex.
```

### Sobrescrever com --force

```bash
tex-cli render greeting /tmp/greeting.json           # 1ª vez
tex-cli render greeting /tmp/greeting.json --force   # sobrescreve
# → Renderizado (sobrescrito) em ~/tex-tmp/output/greeting.tex.
```

### Modo interativo

```bash
tex-cli render
# ? Nome do template: › greeting
# ? Caminho do arquivo JSON (- para stdin): › /tmp/greeting.json
# ? Modo dry-run? [y/N] › n
# → Renderizado em ~/tex-tmp/output/greeting.tex.
```

---

## 5. Falhas esperadas (validam contratos)

### Template ausente

```bash
tex-cli render nao-existe /tmp/greeting.json
# stderr: Template 'nao-existe' não existe em ~/tex-tmp/templates.
# exit 20
```

### JSON malformado

```bash
echo 'not json' > /tmp/bad.json
tex-cli render greeting /tmp/bad.json
# stderr: JSON inválido em /tmp/bad.json: expected value at line 1 column 1.
# exit 31
```

### JSON top-level não é objeto

```bash
echo '["array"]' > /tmp/arr.json
tex-cli render greeting /tmp/arr.json
# stderr: JSON inválido em /tmp/arr.json: esperado objeto no topo, recebido array.
# exit 31
```

### Template referencia variável ausente

```bash
echo '{}' > /tmp/empty.json
tex-cli render greeting /tmp/empty.json
# stderr: Falha ao renderizar template 'greeting': Variable `nome` not found in context...
# exit 30
```

### Sobrescrita sem --force em não-TTY

```bash
tex-cli render greeting /tmp/greeting.json                # OK
tex-cli render greeting /tmp/greeting.json < /dev/null    # sem TTY
# stderr: Arquivo <...>/greeting.tex já existe. Use --force ou execute em terminal interativo.
# exit 15
```

---

## 6. Testes

Suite completa (specs 001 + 002 + 003):

```bash
docker run --rm --name tex-cli \
  -v "$(pwd):/src" -w /src \
  tex-cli cargo test --all
```

Só testes da spec 003:

```bash
docker run --rm -v "$(pwd):/src" -w /src tex-cli \
  cargo test --test cli_render
```

Unit tests do módulo render:

```bash
docker run --rm -v "$(pwd):/src" -w /src tex-cli \
  cargo test --lib render
```

Higiene (fmt + clippy) — mesmo comando das specs anteriores.

---

## 7. Baseline de performance esperada

Alvos do plan:

| Métrica                                          | Alvo    | Notas                          |
|--------------------------------------------------|---------|--------------------------------|
| `render` com template ~10 KB + 10 vars           | <100 ms | SC-001                         |
| `render --dry-run` mesmo template                | <50 ms  | Sem I/O de gravação            |
| Ciclo `cargo test --all` incremental             | <3 s    | Novo binário `cli_render`      |

Comando de smoke bench:

```bash
# Após setup (§3):
for run in 1 2 3 4 5; do
  START=$(date +%s%N)
  ./target/release/tex-cli render greeting /tmp/greeting.json > /dev/null
  END=$(date +%s%N)
  echo "run $run: $(( (END - START) / 1000000 )) ms"
done
```

---

## 7b. Baselines medidos (T031)

Executado em `2026-07-10` dentro do container Docker sancionado,
binário release, template `examples/templates/artigo-basico.tex`
(~2.5 KB, 8 variáveis) + JSON `examples/data/artigo-basico.json`:

| Métrica                                                | Alvo    | Medido    |
|--------------------------------------------------------|---------|-----------|
| SC-001 · `render` template ~2.5 KB, 8 vars              | <100 ms | **2 ms**  |
| SC-005 · `--dry-run` byte-a-byte igual ao arquivo       | Sim     | Sim (teste `render_dry_run_byte_identical_to_written_file`) |
| Build inicial `cargo build --release` (cold)             | —       | ~20 s     |
| Ciclo `cargo test --all` (target/ quente)                | —       | ~2 s      |
| `cargo fmt --check` + `clippy -D warnings`              | zero   | zero     |

SC-002 (script CI não-interativo) validado end-to-end no §4 (init →
templates add → render → verificar output).

## 8. Troubleshooting

| Sintoma                                              | Causa provável                              | Ação                                           |
|------------------------------------------------------|---------------------------------------------|------------------------------------------------|
| `Variable ... not found in context` (exit 30)        | Placeholder no template sem chave no JSON   | Adicione a chave ao JSON ou use `\| default(value="...")`. |
| `expected object at line N column M` (exit 31)       | JSON não tem `{...}` no topo                | Envolva em objeto.                             |
| `Nenhum config encontrado` (exit 10)                 | Config ausente                              | Rode `tex-cli init`.                           |
| `Diretório de templates '...' não existe` (exit 22)  | `paths.templates_dir` inválido              | `mkdir` ou `tex-cli config set paths.templates_dir <path>`. |
| Broken pipe em `render --dry-run \| head`            | Follow-up herdado da spec 002 (SIGPIPE)     | Aguarda fix em spec futura.                    |
| `Arquivo ... já existe` (exit 15)                    | Overwrite sem `--force` em não-TTY          | Adicione `--force` ou rode em TTY.             |
| Caracteres LaTeX escapados errado no output          | Template não escapa `&`, `%`, `$` etc.      | Ajuste o template — autoescape=false por design (D-07). |
