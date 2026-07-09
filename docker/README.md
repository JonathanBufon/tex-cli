# Docker workflow para `tex-cli`

O container Debian abaixo é o ambiente sancionado para rodar a suíte
de testes de `tex-cli` — em especial os testes de integração que
dependem de `tectonic` estar disponível no `PATH`.

## Build da imagem

```bash
docker build -t tex-cli -f docker/Dockerfile .
```

O build inclui `tectonic` (do repositório Debian bookworm) e a toolchain
Rust stable via `rustup`.

## Rodar a suíte completa

```bash
docker run --rm --name tex-cli \
  -v $(pwd):/src -w /src \
  tex-cli cargo test --all
```

O bind-mount `-v $(pwd):/src` compartilha o código-fonte com o
container, então mudanças locais são refletidas imediatamente sem
rebuild da imagem.

## Rodar um teste específico

```bash
docker run --rm --name tex-cli \
  -v $(pwd):/src -w /src \
  tex-cli cargo test --test cli_init
```

## Higiene: fmt + clippy dentro do container (dev flow padrão)

Antes de qualquer commit / PR:

```bash
docker run --rm --name tex-cli \
  -v $(pwd):/src -w /src \
  tex-cli sh -lc "cargo fmt --all -- --check && cargo clippy --all-targets --all-features -- -D warnings"
```

Ambos são gates hard: `fmt` reprova se algum arquivo diverge do estilo;
`clippy -D warnings` reprova qualquer lint. Rode `cargo fmt --all` sem
`--check` para aplicar as correções antes.

## Copiar artefatos gerados no container para o host

Convenção do projeto: PDFs e outros artefatos gerados dentro do
container são copiados para o diretório de compartilhamento no host:

```bash
docker cp tex-cli:<caminho-no-container> /home/jonathan/Downloads/dockers-sharefiles/<destino>
```

Isso preserva o container isolado e evita instalar dependências
LaTeX no host.

## Testes que não precisam de tectonic

Podem rodar direto no host, sem Docker:

```bash
cargo test --all
```

Só falham se algum teste específico espera `tectonic` no `PATH`.
Esses testes marcados devem ser rotulados como `#[ignore]` no host
ou executados exclusivamente no container.
