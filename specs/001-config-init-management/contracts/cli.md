# Phase 1 — CLI Contract: `tex-cli` (subcomandos desta feature)

**Feature**: `001-config-init-management`
**Date**: 2026-07-09

Este documento é o **contrato observável** dos três subcomandos que
esta feature entrega. É o "spec pública" que testes de integração
validam ponto a ponto. Não descreve implementação — só o que o
usuário/script vê.

---

## Flags globais

Aplicam-se a **todos** os subcomandos desta feature.

| Flag                | Curta | Repetível | Efeito                                             |
|---------------------|-------|-----------|----------------------------------------------------|
| `--verbose`         | `-v`  | Sim       | Sobe nível de log: 0=WARN, 1=INFO, 2=DEBUG, 3+=TRACE (D-05). Logs sempre em **stderr**. |
| `--help`            | `-h`  | Não       | Ajuda gerada pelo `clap`.                          |
| `--version`         | `-V`  | Não       | Versão do binário.                                 |

## Banner (transversal a todos os subcomandos)

Antes de qualquer prompt, warning ou saída útil, o binário emite o
banner "TEX CLI" (asset `assets/banner.txt`, incorporado via
`include_str!`) em **stderr**. É emitido exatamente uma vez por
invocação, na inicialização de `main.rs`, antes do dispatch para o
handler do subcomando.

Regras invariantes:

- Nunca vai para stdout (garante `config show --format=json | jq`
  seguro).
- Nunca é suprimido nesta v1 (nem por `-v 0`, nem por env var).
- Aparece igualmente em `init`, `config show`, `config set`,
  `--version`, `--help` — e em qualquer subcomando futuro sem
  intervenção adicional (single choke point).

Contratos observáveis:

- Executar qualquer subcomando com `2>/dev/null` **remove** o banner
  da tela (banner só existe em stderr).
- Executar qualquer subcomando com `>/dev/null` **mantém** o banner
  visível (banner nunca esteve em stdout).
- Diff byte-a-byte de `tex-cli config show --format=json 2>/dev/null`
  antes e depois de qualquer refactor deve permanecer estável — o
  banner nunca contamina esse fluxo.

---

## `tex-cli init`

### Grammar

```text
tex-cli init [-v ...]
```

### Behavior

1. Resolve caminho do config via `dirs::config_dir()/tex/config.toml`.
2. Se config existente:
   - Prompt `inquire::Confirm`: "Sobrescrever config em `<path>`? [y/N]".
   - Resposta "não" → exit `15` (FR-006, D-09).
3. Prompt `inquire::Text` para `paths.templates_dir` (obrigatório).
   - Expansão de `~` e relativos aplicada (D-04).
   - Se diretório não existe → prompt `Confirm` "Criar `<path>`? [y/N]".
     - "não" → exit `15`.
     - "sim" → cria com `std::fs::create_dir_all`, permissões
       padrão do sistema (não obriga `0600` no diretório do usuário).
4. Prompt `inquire::Text` para `paths.output_dir` (obrigatório).
   - Mesmo tratamento do passo 3 quanto a expansão e criação.
5. Prompt `inquire::Select` para `compiler.engine` com opções:
   `tectonic` (default), `latexmk`, `pdflatex`, `xelatex`, `lualatex`.
6. Verifica presença do binário escolhido via `which::which`:
   - Ausente → mensagem em **stderr** (nível WARN, sempre visível):
     `Aviso: '<engine>' não foi encontrado no PATH. A preferência foi
     salva, instale o binário depois.` — **não** aborta.
7. Se engine escolhida ≠ `tectonic`: mensagem `Aviso: engine '<x>'
   ainda não é executada pelo Tex nesta versão. A preferência foi
   salva.` (FR-019).
8. Grava o config atomicamente (tmpfile + `set_permissions(0o600)` +
   `persist`). Estrutura completa incluindo defaults (D-06).
9. **stdout** final: `Config gravado em: <path-absoluto>`.
10. Exit `0`.

### Exit codes

- `0` sucesso;
- `1` erro genérico (`anyhow`);
- `14` permissão negada ao gravar;
- `15` usuário abortou (Ctrl+C ou resposta "não" a confirmação
  crítica).

### Saída

- **stdout**: apenas a linha final de confirmação (sucesso).
- **stderr**: prompts do `inquire`, warnings, erros.

### Contratos observáveis (testes de integração)

- Ao ser executado em ambiente limpo (sem config), gera arquivo em
  `$XDG_CONFIG_HOME/tex/config.toml` (ou `$HOME/.config/tex/config.toml`
  como fallback) com estrutura canônica (data-model.md).
- Arquivo criado tem modo `0600` — verificável via
  `std::fs::metadata(path).permissions().mode() & 0o777 == 0o600`.
- Executar `init` sobre config existente sem confirmar overwrite
  não altera o arquivo original (byte-a-byte).

---

## `tex-cli config show [--format=<humano|json|toml>] [-v ...]`

### Grammar

```text
tex-cli config show
tex-cli config show --format humano
tex-cli config show --format json
tex-cli config show --format toml
```

Valor default de `--format`: `humano`.

### Behavior

1. Resolve caminho do config.
2. Se config **ausente**: stderr `Nenhum config encontrado. Rode
   'tex-cli init' primeiro.` → exit `10` (FR-011).
3. Se config **corrompido** (TOML inválido ou schema divergente):
   stderr `Config em <path> está inválido: <descrição humana do erro>.
   Rode 'tex-cli init' novamente para recriar.` → exit `11` (FR-012).
4. Formatação de saída em stdout:
   - `humano`: agrupado por seção, campos alinhados. Sem cores
     nesta v1.
   - `json`: `serde_json::to_string_pretty` da struct `Config`.
     Nada de metadata externa. Determinístico (mesma ordem de campos,
     mesma indentação).
   - `toml`: re-serializa via `toml::to_string_pretty` — pode diferir
     do arquivo original em espaçamento se o usuário editou à mão,
     mas semântica idêntica.
5. Exit `0`.

### Exit codes

`0` sucesso; `10` config ausente; `11` config corrompido; `1` erro
genérico.

### Saída

- **stdout**: exclusivamente o payload formatado. Zero linhas de
  cabeçalho/rodapé em `json` e `toml` (para permitir pipe direto).
- **stderr**: apenas erros e logs de verbose.

### Contratos observáveis (testes de integração)

- `tex-cli config show --format json | jq .compiler.engine` retorna
  string entre aspas (não erro de parse).
- Executar `config show` não altera mtime do arquivo de config.
- `--format toml` produz saída que, se salva num arquivo e passada
  para `tex-cli config show`, resulta em saída idêntica.

---

## `tex-cli config set <key> <value> [-v ...]`

### Grammar

```text
tex-cli config set <key> <value>
```

Onde `<key>` MUST ser um dos literais canônicos (data-model.md ›
tabela de mapeamento):

- `paths.templates_dir`
- `paths.output_dir`
- `compiler.engine`
- `compiler.keep_tex`
- `compiler.keep_logs`
- `behavior.ask_output_path_every_time`

### Behavior

1. Resolve caminho do config.
2. Se config ausente: stderr `Nenhum config encontrado. Rode
   'tex-cli init' primeiro.` → exit `10` (FR-020).
3. Parse de `<key>` via `FromStr` do enum `ConfigKey`. Se falhar:
   stderr `Chave desconhecida: '<key>'. Chaves aceitas:\n  - ...`
   (lista as 6 canônicas). → exit `12` (FR-015).
4. Parse de `<value>` conforme tipo esperado (data-model.md):
   - **path**: expande `~` e relativos.
   - **bool**: aceita apenas `true`/`false`. Outro valor: stderr
     `Valor inválido para chave booleana '<key>': '<value>'. Aceito:
     true, false.` → exit `13` (FR-017).
   - **string** (`compiler.engine`): não vazia.
5. Aplica a mudança em cima do `Config` carregado.
6. Warnings não-bloqueantes:
   - Se `<key>` é path e o diretório não existe → stderr `Aviso:
     '<path>' não existe no momento.` (FR-018).
   - Se `<key>` é `compiler.engine` e valor ≠ `tectonic` → stderr
     `Aviso: engine '<x>' ainda não é executada pelo Tex nesta
     versão.` (FR-019).
   - Se `<key>` é `compiler.engine` e binário não está no PATH →
     stderr `Aviso: '<engine>' não está no PATH.`
7. Grava atomicamente (tmpfile + `0600` + `persist`).
8. stdout: `Config atualizado: <key> = <valor-normalizado>`.
9. Exit `0`.

### Exit codes

`0` sucesso; `10` config ausente; `12` chave desconhecida; `13`
valor inválido; `14` permissão negada; `1` erro genérico.

### Saída

- **stdout**: apenas linha final de confirmação.
- **stderr**: warnings + erros + prompts (não há prompts neste
  subcomando — é 100% não-interativo).

### Contratos observáveis (testes de integração)

- Após `tex-cli config set compiler.keep_tex false`, o arquivo de
  config muda **apenas** a linha `keep_tex`; nenhuma outra linha
  (paths, outros campos de compiler, behavior) muda semanticamente.
- `tex-cli config set paths.templates_dir '~/x'` grava um caminho
  **absoluto** expandido, não `~/x` literal.
- `tex-cli config set compiler.keep_tex maybe` retorna exit `13`
  e não modifica o arquivo (verificável comparando hash antes/depois).
- `tex-cli config set foo.bar baz` retorna exit `12` e lista as 6
  chaves aceitas em stderr.

---

## Ausências deliberadas (fora do escopo desta spec)

- Nenhum subcomando para **remover** chave (`config unset`). Fica para
  spec futura se surgir necessidade.
- Nenhum modo interativo alternativo em `config set`. Determinismo é
  requisito (FR-013, SC-006).
- Nenhum `--dry-run`. Considerar se ganhar tração.
- Nenhum `--config <path>` para forçar caminho alternativo. Config é
  sempre em `~/.config/tex/config.toml` nesta v1.
