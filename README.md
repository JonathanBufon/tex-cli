# tex-cli

CLI de terminal, escrita em Rust, que converte dados estruturados em
JSON para documentos LaTeX e compila esses documentos para PDF via
`tectonic` (padrão) ou outro engine plugável. Este binário é a camada
de automação sobre o ecossistema LaTeX — não uma reimplementação dele.

Esta v1 entrega a base da ferramenta: gestão do arquivo de
configuração em `~/.config/tex/config.toml` via três subcomandos
(`init`, `config show`, `config set`). Etapas futuras vão empilhar
sobre essa base os subcomandos `templates`, `render`, `compile` e
`build`.

## Instalação

Requer Rust stable (edição 2021).

```bash
cargo install --path .
```

Depois de instalar, verifique:

```bash
tex-cli --version
```

## Uso básico

### Primeira configuração

Interativo:

```bash
tex-cli init
```

Ou não-interativo (útil em scripts / CI):

```bash
tex-cli init \
  --templates-dir ~/Documents/tex/templates \
  --output-dir    ~/Documents/tex/output \
  --engine        tectonic \
  --create-dirs
```

Flags disponíveis:

- `-t, --templates-dir <path>` — pula o prompt de templates
- `-o, --output-dir <path>` — pula o prompt de output
- `-e, --engine <name>` — pula o prompt de engine (`tectonic`,
  `latexmk`, `pdflatex`, `xelatex`, `lualatex`)
- `--create-dirs` — cria diretórios ausentes sem confirmar
- `--force` — sobrescreve config existente sem confirmar

### Inspecionar o config

```bash
tex-cli config show                    # padrão: humano
tex-cli config show --format json      # pipe direto pro jq
tex-cli config show --format toml      # re-serialização TOML
```

### Alterar uma chave

```bash
tex-cli config set compiler.keep_tex false
tex-cli config set paths.templates_dir ~/nova/pasta
tex-cli config set compiler.engine tectonic
```

Chaves aceitas (dotted-path canônico, sem aliases):

- `paths.templates_dir`
- `paths.output_dir`
- `compiler.engine`
- `compiler.keep_tex`
- `compiler.keep_logs`
- `behavior.ask_output_path_every_time`

### Gerenciar templates

O grupo `templates` administra os arquivos `.tex` que ficam em
`paths.templates_dir`.

```bash
tex-cli templates list                     # padrão: humano
tex-cli templates list --format json       # pipe direto pro jq

tex-cli templates show artigo              # bytes brutos em stdout
tex-cli templates show artigo.tex          # extensão opcional

tex-cli templates add /tmp/novo.tex                  # basename → novo
tex-cli templates add /tmp/x.tex --name relatorio    # nome custom
tex-cli templates add /tmp/x.tex --force             # sobrescreve

tex-cli templates remove artigo --force    # remove sem prompt

tex-cli templates                          # menu interativo (TTY)
```

Contratos importantes:

- `list --format json` produz JSON válido para pipe em `jq` (mesmo
  com diretório vazio → `[]`).
- Cada template carrega `modified_at_epoch` (segundos desde
  UNIX_EPOCH); use `jq '... | strftime("%Y-%m-%dT%H:%M:%SZ")'` para
  formatar em RFC3339.
- `add` valida UTF-8 antes de gravar e faz escrita atômica com
  permissão `0644` (compartilhável via git, ao contrário do config).
- `remove` exige `--force` ou confirmação em terminal interativo.

Exemplos curados de templates estão em [`examples/templates/`](examples/templates/)
com atribuição ao autor original.

### Verbosidade

Flag global `-v` repetível em qualquer subcomando:

```bash
tex-cli -v config show      # INFO
tex-cli -vv init            # DEBUG
tex-cli -vvv config set ... # TRACE
```

Logs vão sempre para stderr; stdout fica limpo para pipes.

## Códigos de saída

| Código | Significado                           |
|--------|---------------------------------------|
| 0      | Sucesso                               |
| 1      | Erro genérico não classificado        |
| 2      | Erro de argumento CLI (via clap)      |
| 10     | Config ausente                        |
| 11     | Config corrompido (TOML inválido)     |
| 12     | Chave desconhecida em `config set`    |
| 13     | Valor inválido (ex.: bool malformado) |
| 14     | Permissão negada ao gravar            |
| 15     | Usuário abortou operação interativa   |
| 20     | Template não existe (`show`/`remove`) |
| 21     | Arquivo do `add` não é UTF-8 válido   |
| 22     | `paths.templates_dir` inexistente     |

## Desenvolvimento

Testes rodam em container Debian com Rust + tectonic:

```bash
docker build -t tex-cli -f docker/Dockerfile .
docker run --rm -v $(pwd):/src -w /src tex-cli cargo test --all
```

Higiene padrão antes de commit:

```bash
docker run --rm -v $(pwd):/src -w /src tex-cli \
  sh -lc "cargo fmt --all -- --check && cargo clippy --all-targets --all-features -- -D warnings"
```

Detalhes em [`docker/README.md`](docker/README.md).

## Documentação de design

- [Constituição do projeto](.specify/memory/constitution.md)
- [Spec da v1 (config init/show/set)](specs/001-config-init-management/spec.md)
- [Plan](specs/001-config-init-management/plan.md)
- [Quickstart](specs/001-config-init-management/quickstart.md)

## Licença

Ver arquivo [`LICENSE`](LICENSE).
