# Phase 1 — Quickstart: Gestão de Templates LaTeX

**Feature**: `002-templates-management`
**Date**: 2026-07-09

Guia rápido para um dev clonar o repo (com spec 001 já mergeada),
buildar, rodar os testes e exercitar os quatro subcomandos + menu
interativo desta feature.

**Pré-requisito duro**: spec 001 (config-init-management) precisa
estar mergeada em `main`. Toda a infra (Cargo.toml, Dockerfile,
banner, `Config::load`, `TexError`, integração assert_cmd) é
reaproveitada.

---

## 1. Pré-requisitos

### Host

- Rust stable via `rustup`.
- Docker (para os testes de integração isolados).
- `git`.

### Docker

Nenhuma pré-instalação. O `docker/Dockerfile` da spec 001 já contém
Rust + tectonic + rustfmt + clippy. Reuso direto.

---

## 2. Build local

Nenhuma nova dep — o Cargo.toml permanece intocado por esta feature
(research D-01 evitou adicionar dep de datas).

```bash
git checkout 002-templates-management
cargo build            # debug
cargo build --release  # otimizado
```

Binário resultante: `target/debug/tex-cli` (ou `target/release/tex-cli`).

### Instalação local (opcional)

```bash
cargo install --path .
```

---

## 3. Preparação: config e templates_dir

Como a feature depende da spec 001, primeiro configura o config:

```bash
mkdir -p ~/tex-tmp/templates ~/tex-tmp/output

tex-cli init \
  --templates-dir ~/tex-tmp/templates \
  --output-dir    ~/tex-tmp/output \
  --engine        tectonic \
  --force
```

Depois cria alguns templates de exemplo:

```bash
cat > ~/tex-tmp/templates/artigo.tex <<'EOF'
\documentclass{article}
\begin{document}
Hello, {{ nome }}.
\end{document}
EOF

cat > ~/tex-tmp/templates/carta.tex <<'EOF'
\documentclass{letter}
\begin{document}
Prezado {{ destinatario }},
\end{document}
EOF
```

---

## 4. Rodar os subcomandos manualmente

### `templates list`

```bash
tex-cli templates list
```

Saída humana esperada:

```text
NOME       TAMANHO   MODIFICADO
artigo         55  2026-07-09 14:30
carta          46  2026-07-09 14:30
```

### `templates list --format=json`

```bash
tex-cli templates list --format=json | jq .
```

Saída esperada:

```json
[
  {
    "name": "artigo",
    "path": "/home/user/tex-tmp/templates/artigo.tex",
    "size_bytes": 55,
    "modified_at_epoch": 1741550055
  },
  {
    "name": "carta",
    "path": "/home/user/tex-tmp/templates/carta.tex",
    "size_bytes": 46,
    "modified_at_epoch": 1741550055
  }
]
```

Convertendo epoch para data legível via `jq`:

```bash
tex-cli templates list --format=json \
  | jq '.[] | (.name + " " + (.modified_at_epoch | strftime("%Y-%m-%dT%H:%M:%SZ")))'
```

### `templates show <nome>`

```bash
tex-cli templates show artigo
tex-cli templates show artigo.tex   # extensão opcional
```

Ambos imprimem o conteúdo bruto do `artigo.tex` no stdout.

Pipe direto pra $EDITOR ou `less`:

```bash
tex-cli templates show artigo | less
```

### `templates add`

```bash
# Criar um template de teste
cat > /tmp/relatorio.tex <<'EOF'
\documentclass{report}
\begin{document}
Título: {{ titulo }}
\end{document}
EOF

# Adicionar ao repositório central
tex-cli templates add /tmp/relatorio.tex
# → stdout: Template 'relatorio' adicionado em /home/user/tex-tmp/templates/relatorio.tex.

# Adicionar com nome custom
tex-cli templates add /tmp/relatorio.tex --name relatorio-v2
# → Template 'relatorio-v2' adicionado em ...

# Sobrescrever existente (sem prompt)
tex-cli templates add /tmp/relatorio.tex --force
# → Template 'relatorio' sobrescrito em ...
```

Falha esperada com arquivo binário:

```bash
head -c 100 /dev/urandom > /tmp/binario.tex
tex-cli templates add /tmp/binario.tex
# stderr: Arquivo '/tmp/binario.tex' não é texto UTF-8 válido: ...
# exit 21
```

### `templates remove`

```bash
# Remover interativo (pede confirmação em TTY)
tex-cli templates remove relatorio-v2
# ? Remover template 'relatorio-v2'? [y/N]

# Remover não-interativo (script/CI)
tex-cli templates remove relatorio-v2 --force
# → stdout: Template 'relatorio-v2' removido de ...
```

Falha esperada com nome inexistente:

```bash
tex-cli templates remove nao-existe
# stderr: Template 'nao-existe' não existe em /home/user/tex-tmp/templates.
# exit 20
```

### Modo interativo `templates`

```bash
tex-cli templates
# ? O que fazer com os templates?
# > Listar templates
#   Inspecionar template
#   Adicionar template
#   Remover template
#   Sair
```

Escolhe uma ação, executa, retorna. Sem loop (research D-09).

---

## 5. Testes locais

Suite unitária sem dep de FS externa:

```bash
cargo test --lib
```

Testes de integração da feature (drivam o binário compilado via
`assert_cmd`, sem depender de tectonic):

```bash
cargo test --test cli_templates_list
cargo test --test cli_templates_show
cargo test --test cli_templates_add
cargo test --test cli_templates_remove
```

Suite completa:

```bash
cargo test --all
```

---

## 6. Testes no container Docker

Mesmo container `tex-cli` da spec 001. Nenhuma mudança:

```bash
docker build -t tex-cli -f docker/Dockerfile .

docker run --rm --name tex-cli \
  -v "$(pwd):/src" -w /src \
  tex-cli cargo test --all
```

Higiene padrão:

```bash
docker run --rm --name tex-cli \
  -v "$(pwd):/src" -w /src \
  tex-cli sh -lc "cargo fmt --all -- --check && \
                  cargo clippy --all-targets --all-features -- -D warnings"
```

---

## 7. Fluxo de trabalho dia-a-dia

```text
1. Editar código no host (IDE preferido, cargo check rápido).
2. cargo test --lib          → smoke local (unit tests)
3. cargo test --test cli_templates_*   → integração da feature
4. cargo clippy -- -D warnings
5. cargo fmt --all -- --check
6. docker run ... cargo test --all   → validação canônica
7. Commit
```

---

## 8. Baseline de performance esperada

Alvos declarados no plan:

| Métrica                                  | Alvo    | Notas                            |
|------------------------------------------|---------|----------------------------------|
| `templates list --format=json` (100 files)| <100 ms | SC-002                           |
| `templates show` (arquivo < 1 MB)        | <100 ms | SC-004 implícito                 |
| `templates add` (arquivo < 100 KB)       | <100 ms | Alinhado com `config set`        |
| `templates remove`                       | <100 ms | Uma chamada `remove_file`        |

Comando de smoke test manual:

```bash
# Preparar 100 templates
mkdir -p /tmp/many-templates
for i in $(seq -w 1 100); do
  echo "\\documentclass{article}\\begin{document}$i\\end{document}" \
    > /tmp/many-templates/tpl$i.tex
done

# Reconfigurar
tex-cli config set paths.templates_dir /tmp/many-templates

# Medir
START=$(date +%s%N)
tex-cli templates list --format=json > /dev/null
END=$(date +%s%N)
echo "list 100 templates: $(( (END - START) / 1000000 )) ms"
```

---

## 9. Troubleshooting

| Sintoma                                              | Causa provável                       | Ação                                                     |
|------------------------------------------------------|--------------------------------------|----------------------------------------------------------|
| `Nenhum config encontrado. Rode 'tex-cli init'…`     | Config ausente                       | Roda `tex-cli init` primeiro (spec 001).                 |
| `Diretório de templates '…' não existe.`            | `templates_dir` do config aponta pra nada | `mkdir <path>` ou `tex-cli config set paths.templates_dir …`. |
| `Template '…' não existe em …`                       | Nome errado ou template deletado     | `tex-cli templates list` para ver o que há.              |
| `Arquivo '…' não é texto UTF-8 válido`               | Fonte binária ou latin1              | Converter com `iconv` antes: `iconv -f LATIN1 -t UTF-8`. |
| `Menu interativo de templates requer terminal.`      | Rodou `tex-cli templates` sem TTY    | Use subcomando explícito (`list`/`show`/etc.).           |
| Exit 15 em CI ao rodar `add` sobre template existente | Falta `--force` em ambiente não-TTY  | Adicionar `--force` no comando.                          |
