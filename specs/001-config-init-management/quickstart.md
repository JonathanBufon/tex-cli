# Phase 1 — Quickstart: Configuração Inicial e Gestão do Config do Tex

**Feature**: `001-config-init-management`
**Date**: 2026-07-09

Guia rápido para um dev clonar o repo, buildar, rodar os testes
(localmente e no container Debian) e exercitar os três subcomandos
desta feature.

---

## 1. Pré-requisitos

### Host

- Rust stable (via `rustup`). Verifique: `rustc --version`.
- Docker instalado (para os testes de integração isolados).
- `git`.

### Docker

Nenhuma pré-instalação. O Dockerfile em `docker/Dockerfile` cuida do
resto (Rust + `tectonic` em Debian bookworm).

---

## 2. Build local

```bash
git clone <repo>
cd tex
cargo build            # debug
cargo build --release  # otimizado
```

Binário resultante: `target/debug/tex-cli` (ou `target/release/tex-cli`).

### Instalação local (opcional)

```bash
cargo install --path .
```

Isso coloca `tex-cli` em `~/.cargo/bin/`. Certifique-se de que esse
caminho está no `PATH`.

---

## 3. Rodar os subcomandos manualmente

### `init` (primeira execução)

```bash
tex-cli init
```

Fluxo esperado (interativo):

```text
? Diretório de templates LaTeX: › ~/Documents/tex/templates
? Diretório padrão de saída dos PDFs: › ~/Documents/tex/output
? Compilador LaTeX: › tectonic
Config gravado em: /home/<you>/.config/tex/config.toml
```

Verifica permissões:

```bash
stat -c '%a' ~/.config/tex/config.toml   # deve mostrar 600
```

### `config show`

```bash
tex-cli config show                     # humano
tex-cli config show --format json       # para scripts
tex-cli config show --format toml       # re-serializado
```

Exemplo com `jq`:

```bash
tex-cli config show --format json | jq '.compiler.engine'
# "tectonic"
```

### `config set`

```bash
tex-cli config set paths.output_dir ~/Documents/tex/output-2026
tex-cli config set compiler.keep_tex false
tex-cli config set compiler.engine latexmk    # aviso: não suportado
```

Falhas esperadas:

```bash
tex-cli config set foo.bar baz
# stderr: Chave desconhecida: 'foo.bar'. Chaves aceitas: ...
# exit 12

tex-cli config set compiler.keep_tex maybe
# stderr: Valor inválido para chave booleana 'compiler.keep_tex': 'maybe'.
# exit 13
```

---

## 4. Testes locais

```bash
cargo test                    # unit + integração no host
cargo test --lib              # só unit tests
cargo test --test cli_init    # só integração do init
```

Testes que verificam ausência de tectonic no PATH usam
`assert_cmd::Command::env("PATH", "/tmp/empty-path-for-test")`
(criado com `assert_fs`) — não dependem do estado do host.

---

## 5. Testes no container Debian (isolado)

O container é o ambiente **canônico** para testes de integração
envolvendo binários LaTeX. O host não precisa ter `tectonic` instalado.

### Build da imagem

```bash
docker build -t tex-cli -f docker/Dockerfile .
```

### Rodar toda a suíte de testes

```bash
docker run --rm \
  --name tex-cli \
  -v "$(pwd):/src" \
  -w /src \
  tex-cli cargo test --all
```

Explicação:

- `-v $(pwd):/src` monta o workspace no container.
- `-w /src` define diretório de trabalho.
- `--name tex-cli` fixa nome (compatível com a memória do projeto —
  usado por `docker cp`).
- Base image já vem com `tectonic` e toolchain Rust.

### Simular ausência de tectonic dentro do container

Sobrescreve o PATH na chamada de teste específica:

```bash
docker run --rm --name tex-cli -v "$(pwd):/src" -w /src tex-cli \
  bash -c 'PATH="/tmp/empty" cargo test --test cli_init test_init_warns_when_tectonic_missing'
```

### Recuperar artefatos do container

Convenção (memória `workflow-docker-testing`): host destination
padrão é `/home/jonathan/Downloads/dockers-sharefiles/`.

Nesta feature ainda não geramos PDFs, mas o padrão para as próximas
specs é:

```bash
docker cp tex-cli:/tmp/parecer.pdf \
  /home/jonathan/Downloads/dockers-sharefiles/parecer.pdf
```

---

## 6. Fluxo de trabalho recomendado no dia-a-dia

```text
1. Editar código no host (IDE preferido, cargo check rápido).
2. cargo test --lib          → smoke local (unit tests)
3. cargo clippy -- -D warnings
4. cargo fmt
5. docker run ... cargo test --all   → validação canônica
6. Commit
```

Se o passo 2 passar mas o passo 5 falhar, a diferença
provavelmente está em algo que depende de binário externo (`tectonic`,
`which`) — o container é a verdade.

---

## 7. Debug com verbose

Todos os subcomandos aceitam `-v` repetível:

```bash
tex-cli -v config show           # info level
tex-cli -vv config show          # debug level
tex-cli -vvv init                # trace level (verboso)
```

Logs vão para **stderr**. Combine com redirecionamento se precisar
capturar:

```bash
tex-cli -vv config set compiler.engine tectonic 2> debug.log
```

---

## 8. Baseline de performance e CI (T034)

Medições feitas em `2026-07-09` dentro do container Docker sancionado,
binário release, sobre config canônico (~200 bytes):

| Métrica                                           | Alvo    | Medido |
|---------------------------------------------------|---------|--------|
| SC-002 · `config show --format json` (wall-clock) | <100 ms | 1 ms   |
| Ciclo `cargo test --all` (target/ quente)         | —       | ~2 s   |
| Build inicial (`cargo build --release`)            | —       | ~16 s  |

SC-006 (script CI não-interativo) validado end-to-end no mesmo
container:

```bash
tex-cli init \
  --templates-dir /tmp/templates \
  --output-dir /tmp/output \
  --engine tectonic \
  --create-dirs

tex-cli config show --format json | jq .compiler.keep_tex   # true
tex-cli config set compiler.keep_tex false
tex-cli config show --format json | jq .compiler.keep_tex   # false
```

Nenhum passo pediu input interativo; todos os subcomandos leem/escrevem
com stdout limpo.

## 9. Troubleshooting

| Sintoma                                                       | Causa provável                                | Ação                                                       |
|---------------------------------------------------------------|-----------------------------------------------|------------------------------------------------------------|
| `Nenhum config encontrado. Rode 'tex-cli init' primeiro.`    | Config ausente                                | Rode `tex-cli init`.                                       |
| `Config em <path> está inválido: ...`                        | TOML corrompido (edição manual quebrada)      | Rode `tex-cli init` (com backup manual do arquivo antes).  |
| `Chave desconhecida: '<key>'.`                                | Chave não é dotted-path canônico              | Ver lista em `contracts/cli.md` ou usar `--help`.          |
| `Valor inválido para chave booleana '...': '<v>'`             | Passou `1`, `yes`, `on`, etc.                 | Use `true` ou `false` (case-sensitive).                    |
| `permission denied` ao gravar config                          | `~/.config/tex/` não gravável                 | Ajuste permissões do diretório home.                       |
| Testes falham no host mas passam no Docker                    | Host tem `tectonic` de versão diferente, ou não tem | Rode sempre o container para validação final.             |
