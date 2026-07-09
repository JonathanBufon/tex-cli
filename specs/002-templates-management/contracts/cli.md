# Phase 1 — CLI Contract: `tex-cli templates` (subcomandos desta feature)

**Feature**: `002-templates-management`
**Date**: 2026-07-09

Este documento é o **contrato observável** dos quatro subcomandos +
menu interativo que esta feature entrega. É o "spec pública" que
testes de integração validam ponto a ponto.

Herdado da spec 001 (não repetido aqui): banner em stderr,
`-v/--verbose` global, `--help`/`--version`, exit codes 0/1/2 padrão.

---

## Flags globais herdadas da spec 001

Aplicam-se transversalmente. Zero mudanças.

| Flag       | Curta | Repetível | Efeito                             |
|------------|-------|-----------|------------------------------------|
| `--verbose`| `-v`  | Sim       | Nível de log: WARN/INFO/DEBUG/TRACE|
| `--help`   | `-h`  | Não       | Ajuda do clap                      |
| `--version`| `-V`  | Não       | Versão do binário                  |

## Banner (herdado, invariante)

Emitido em stderr no início de toda invocação, incluindo dos quatro
subcomandos deste grupo e do modo interativo. Nunca em stdout.
`config show --format=json | jq …` continua seguro.

---

## `tex-cli templates list [--format=<humano|json>]`

### Grammar

```text
tex-cli templates list
tex-cli templates list --format humano
tex-cli templates list --format json
```

Valor default de `--format`: `humano`.

### Behavior

1. Carrega config via `Config::load(config_file_path()?)?`.
   - Config ausente → exit `10`, stderr `Nenhum config encontrado.
     Rode 'tex-cli init' primeiro.` (herda spec 001).
   - Config corrompido → exit `11` (herda spec 001).
2. Lê `paths.templates_dir` do config.
3. Chama `list_templates(dir)`.
   - Dir inexistente → exit `22`, stderr
     `Diretório de templates '<path>' não existe. Rode 'tex-cli
     config set paths.templates_dir <path>' ou crie o diretório.`
   - Dir sem permissão de leitura → exit `14` (PermissionDenied).
4. Ordena por `name` alfabeticamente.
5. Formata:
   - `humano`: tabela com colunas `NOME` / `TAMANHO` / `MODIFICADO`.
     Se lista vazia → stdout `Nenhum template encontrado em <path>.`
     e exit `0`.
   - `json`: array JSON. Determinístico. Vazio = `[]`.
6. Exit `0`.

### Exit codes

`0` sucesso; `10` config ausente; `11` config corrompido; `14`
templates_dir sem permissão de leitura; `22` templates_dir
inexistente; `1` erro genérico.

### Saída

- **stdout**: exclusivamente o payload formatado (JSON puro ou
  tabela humana). Zero prefixo/rodapé no `json`.
- **stderr**: banner + logs de `-v`.

### Contratos observáveis (testes de integração)

- `tex-cli templates list --format=json | jq empty` retorna sucesso
  em 100% dos casos (SC-006).
- `tex-cli templates list --format=json | jq '.[].name'` retorna
  strings entre aspas.
- Rodar `list` sobre um diretório com um `.tex` + um `.bib` + um
  subdir mostra somente o `.tex` na saída.
- `list` não altera mtime de nenhum arquivo.

---

## `tex-cli templates show <nome>`

### Grammar

```text
tex-cli templates show <nome>
```

`<nome>` pode incluir ou omitir `.tex`. Ver research D-02.

### Behavior

1. Carrega config (exit 10/11 herdados).
2. Lê `paths.templates_dir`.
3. `resolve_template(dir, name)`:
   - Template não existe → exit `20`, stderr `Template '<name>'
     não existe em <path>.`
4. Lê o arquivo e escreve bytes brutos em stdout via
   `io::stdout().write_all(&bytes)?` (não passa por `println!` — evita
   qualquer transformação).
5. Exit `0`.

### Exit codes

`0` sucesso; `10` config ausente; `11` config corrompido; `14`
permissão; `20` template ausente; `22` templates_dir inexistente;
`1` erro genérico.

### Saída

- **stdout**: conteúdo bruto do arquivo, byte-por-byte.
- **stderr**: banner + logs.

### Contratos observáveis

- `diff <(tex-cli templates show artigo) <path-do-artigo.tex>` retorna
  zero (empty).
- `tex-cli templates show artigo | wc -c` == tamanho do arquivo em
  bytes.
- `tex-cli templates show <nome-inexistente>` → exit `20` e stdout
  vazio.

---

## `tex-cli templates add <arquivo> [--name <nome>] [--force]`

### Grammar

```text
tex-cli templates add <arquivo>
tex-cli templates add <arquivo> --name meu-template
tex-cli templates add <arquivo> --force
tex-cli templates add <arquivo> --name x --force
```

### Behavior

1. Carrega config (exit 10/11).
2. Lê `paths.templates_dir` (exit 22 se ausente).
3. Determina `dest_name`:
   - Se `--name` fornecido → usar.
   - Senão → `basename(source).strip_suffix(".tex").unwrap_or_else(basename)`.
4. Lê `arquivo` para memória (exit `1` genérico se I/O falhar).
5. Valida UTF-8 via `str::from_utf8`:
   - Falha → exit `21`, stderr `Arquivo '<source>' não é texto UTF-8
     válido: <detail>.`.
6. Verifica se `dir/{dest_name}.tex` já existe:
   - Existe + `--force` → prossegue (sobrescreve).
   - Existe + sem `--force` + TTY → `inquire::Confirm` "Sobrescrever
     template '<name>'? [y/N]"; resposta "não" → exit `15`.
   - Existe + sem `--force` + sem TTY → exit `15`,
     stderr `Template '<name>' já existe. Use --force para
     sobrescrever ou rode em terminal interativo.`.
7. `save_atomic_bytes(&bytes, dir/{dest_name}.tex, mode = 0o644)`:
   - Falha de permissão → exit `14`.
8. stdout: `Template '<dest_name>' adicionado em <caminho-absoluto>.`
   (ou `Template '<dest_name>' sobrescrito em <caminho>.` se
   `overwrote_existing == true`).
9. Exit `0`.

### Exit codes

`0` sucesso; `10` config ausente; `11` config corrompido; `14`
permissão; `15` user-aborted; `21` UTF-8 inválido; `22` templates_dir
inexistente; `1` erro genérico (source path não existe etc.).

### Saída

- **stdout**: apenas a linha final de confirmação.
- **stderr**: banner + prompt de overwrite + logs.

### Contratos observáveis

- Após `add /tmp/x.tex --name y` bem-sucedido:
  - Arquivo `<templates_dir>/y.tex` existe.
  - Modo `& 0o777 == 0o644`.
  - Conteúdo idêntico a `/tmp/x.tex`.
- `add` interrompido (SIGKILL entre etapas) não deixa nem
  o arquivo antigo corrompido nem um `.tmp*` órfão visível — o
  tempfile é sempre reaproveitado ou removido pelo drop.
- `add` sobre arquivo binário → exit `21`, template não é criado.
- `add --force` sobre template existente sobrescreve sem prompt.

---

## `tex-cli templates remove <nome> [--force]`

### Grammar

```text
tex-cli templates remove <nome>
tex-cli templates remove <nome> --force
```

### Behavior

1. Carrega config (exit 10/11).
2. Lê `paths.templates_dir` (exit 22 se ausente).
3. `resolve_template(dir, name)`:
   - Template não existe → exit `20`.
4. Decisão de remover:
   - `--force` → remove.
   - Sem `--force` + TTY → `inquire::Confirm` "Remover template
     '<name>'? [y/N]"; "não" → exit `15`.
   - Sem `--force` + sem TTY → exit `15`, stderr `Template '<name>'
     não removido: use --force ou execute em terminal interativo.`.
5. `fs::remove_file(path)?` (exit `14` em permissão).
6. stdout: `Template '<name>' removido de <caminho-absoluto>.`
7. Exit `0`.

### Exit codes

`0` sucesso; `10` config ausente; `11` config corrompido; `14`
permissão; `15` user-aborted; `20` template ausente; `22`
templates_dir inexistente; `1` erro genérico.

### Contratos observáveis

- `remove --force` sobre template existente → arquivo desaparece,
  exit `0`.
- `remove` sem `--force` em não-TTY sobre template existente → exit
  `15`, arquivo permanece intacto (mesmo hash).
- `remove` sobre nome inexistente → exit `20`, nenhum arquivo tocado.

---

## `tex-cli templates` (sem subcomando, modo interativo)

### Grammar

```text
tex-cli templates
```

### Behavior

1. Carrega config (exit 10/11).
2. Verifica se stdin é TTY (`IsTerminal`).
   - Não é TTY → exit `1` com stderr
     `Menu interativo de templates requer terminal. Use um subcomando
      explícito: tex-cli templates list|show|add|remove.`
3. Abre `inquire::Select` com 5 opções:
   - "Listar templates"
   - "Inspecionar template"
   - "Adicionar template"
   - "Remover template"
   - "Sair"
4. **One-shot** (research D-09): executa a ação escolhida, retorna.
   Não volta ao menu.
5. Para "Inspecionar" e "Remover": prompt adicional para o nome
   (`inquire::Select` populado por `list_templates`).
6. Para "Adicionar": prompt para path do arquivo de origem
   (`inquire::Text`) e para o nome final (com sugestão baseada no
   basename).
7. Executa a ação como se fosse o subcomando direto (mesmo path de
   código).
8. Exit code = mesmo do subcomando executado.
9. "Sair" → exit `0`.
10. Ctrl+C em qualquer ponto → exit `15`.

### Exit codes

Mesmos dos subcomandos executados internamente, + `1` para "menu
requer TTY".

### Contratos observáveis

- Sem TTY, o comando retorna `1` sem travar.
- Em TTY, "Sair" retorna `0`.
- Em TTY, escolher "Listar" produz saída idêntica a
  `tex-cli templates list`.

---

## Ausências deliberadas (fora do escopo desta spec)

- Nenhum `templates rename` — usuário faz `add` + `remove` se
  quiser trocar nome. Anotado para spec futura se surgir demanda.
- Nenhum `templates edit` — chamar `$EDITOR` está fora do escopo;
  usuário edita diretamente com o editor de sua preferência.
- Nenhum suporte a `.tex.j2` / `.template` / outras extensões nesta v1.
- Nenhum suporte a subdiretórios / hierarquia de templates.
- Nenhum suporte a "template projects" multi-arquivo (`unotex.cls`
  + subdirs) — decisão registrada na discussão de integração com
  Eranot/Unotex (ver PR body).
- Nenhum `--format=toml` no `list` — TOML não é natural para arrays
  de objetos com data.
- Nenhum `--dry-run`. Considerar se ganhar tração.
