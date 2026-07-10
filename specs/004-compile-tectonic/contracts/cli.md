# Phase 1 — CLI Contract: `tex-cli compile`

**Feature**: `004-compile-tectonic`
**Date**: 2026-07-10

Contrato observável do subcomando `compile` + modo interativo.
Herdado das specs 001–003 (não repetido): banner em stderr,
`-v/--verbose`, `--help/--version`, exit codes 0/1/2/10/11/14/15/20/22/30/31.

---

## `tex-cli compile <tex-file> [--output <path>] [--engine <name>] [--keep-tex|--no-keep-tex] [--keep-logs|--no-keep-logs] [--force]`

### Grammar

```text
tex-cli compile <tex-file>
tex-cli compile /tmp/artigo.tex
tex-cli compile ./out/artigo.tex --output /tmp/final.pdf
tex-cli compile artigo.tex --engine latexmk
tex-cli compile artigo.tex --keep-tex --keep-logs
tex-cli compile artigo.tex --no-keep-tex
tex-cli compile artigo.tex --force
tex-cli compile artigo.tex --engine tectonic --output /tmp/x.pdf --keep-logs --force
```

Arg positional **obrigatório**:

- `<tex-file>`: path do `.tex` fonte (absoluto ou relativo).
  Aceita `~` e caminhos relativos (expandidos via
  `paths::expand_user_path`).

Flags:

| Flag                  | Curta | Efeito                                                       |
|-----------------------|-------|--------------------------------------------------------------|
| `--output <path>`     | `-o`  | Path custom do PDF final. Aceita `~`/relativos.              |
| `--engine <name>`     | `-e`  | Sobrescreve `compiler.engine` do config só nesta invocação.  |
| `--keep-tex`          | —     | Copia `.tex` fonte para `output_dir` ao final.               |
| `--no-keep-tex`       | —     | Não copia (sobrescreve `compiler.keep_tex=true` do config).  |
| `--keep-logs`         | —     | Copia `.log` do engine para `output_dir` ao final.           |
| `--no-keep-logs`      | —     | Não copia.                                                   |
| `--force`             | —     | Sobrescreve PDF existente sem prompt.                         |

Flags conflitantes (`--keep-tex --no-keep-tex` OU `--keep-logs
--no-keep-logs`) → exit `2` (erro de arg via clap).

### Behavior

1. Carrega config (10/11).
2. Verifica que `<tex-file>` existe → senão exit `1` (I/O NotFound
   genérico) com stderr `arquivo <path> não encontrado`.
3. Resolve engine:
   - `--engine <name>` presente → parse via `SupportedEngine::from_str`;
     falha → exit `42` (`EngineNotSupported`) com stderr listando
     aceitos.
   - Ausente → mesmo parse com `config.compiler.engine`; falha →
     exit `42`.
4. Valida instalação: `which::which(engine.binary_name())` → falha
   → exit `41` (`EngineNotInstalled`) com stderr `Engine '<name>'
   não está instalado no PATH. Instale-o antes ou use --engine
   <outro>.`.
5. Resolve output PDF:
   - `--output <path>` presente → expand_user_path.
   - Ausente → `config.paths.output_dir/<basename>.pdf`.
6. Resolve `keep_tex`/`keep_logs` (flag → config default).
7. Prepara temp dir via `TempDir::new()`, copia `.tex` fonte pra
   dentro.
8. Cronomera com `Instant::now()`.
9. Executa engine (cwd=temp, args do D-03). Com `-vv`: streaming
   pro terminal. Sem: silent.
10. Após execução:
    - Se `ExitStatus::success()` **E** PDF existe em
      `<temp>/<basename>.pdf`: sucesso.
    - Senão: monta `log_tail` (últimas 30 linhas de
      `<temp>/<basename>.log`, ou fallback pro stderr do child),
      retorna `TexError::CompileFailed` → exit `40`.
11. Se sucesso:
    - Verifica overwrite: destino existe + `!--force`:
      TTY → prompt `inquire::Confirm` "Sobrescrever `<path>`?"
      default "não"; "não" → exit `15`.
      Não-TTY → exit `15` sem prompt.
    - `atomic::write_atomic(&output_pdf, &pdf_bytes, 0o644)`.
    - Se `keep_tex`: copia `.tex` para
      `<output_dir>/<basename>.tex` (write atômico, 0o644).
    - Se `keep_logs`: idem pro `.log`.
    - TempDir vai fora de scope → Drop apaga tudo.
12. stdout: uma linha
    `PDF gerado em <path>. Compilação levou X.Ys.` (ou
    `PDF gerado (sobrescrito) em <path>. Compilação levou X.Ys.`).
13. Exit `0`.

### Exit codes

- `0` sucesso.
- `1` erro genérico (`.tex` não encontrado, I/O sem categoria).
- `2` erro de arg CLI (flags conflitantes).
- `10` config ausente.
- `11` config corrompido.
- `14` permissão negada.
- `15` user-aborted.
- `40` falha de compilação (engine retornou erro).
- `41` engine não instalado.
- `42` engine não suportado.

### Saída

- **stdout**: exclusivamente a linha `PDF gerado …. Compilação
  levou X.Ys.` (sucesso) OU nada (falha — tudo vai pra stderr).
- **stderr**: banner + prompts (só TTY + overwrite) + logs de
  verbose + mensagens de erro incluindo o `log_tail` embutido no
  Display de `CompileFailed`.

### Contratos observáveis (testes de integração)

- `compile <tex>` bem-sucedido produz arquivo cujo primeiro
  argumento de `std::fs::read` retorna bytes iniciando com `%PDF-`
  (magic bytes de PDF).
- Modo do PDF gravado = `0o644`.
- `--engine <novo>` **não altera** `.compiler.engine` no config
  file (verificável via `config show --format json | jq
  .compiler.engine` antes e depois).
- `.tex` com erro de sintaxe: exit `40`, stderr menciona o path e
  contém trecho do log; PDF de destino não existe (ou byte-a-byte
  intacto se já existia).
- `--engine foo` (fora canônico): exit `42`, stderr lista os 5
  aceitos, nenhuma compilação iniciada.
- `PATH=/tmp/empty` + engine=tectonic: exit `41`, stderr menciona
  "não está instalado".
- `--keep-tex --keep-logs`: `<output_dir>/<basename>.tex` e
  `<output_dir>/<basename>.log` existem após comando.
- `--no-keep-tex --no-keep-logs`: só `.pdf` existe; qualquer `.tex`
  ou `.log` pré-existente com o mesmo basename é preservado (não
  apagamos arquivos do usuário).
- Zero artefato do TempDir permanece no filesystem após comando
  (validável por snapshot `/tmp`).
- `--force` sobre PDF existente sobrescreve sem prompt.

---

## `tex-cli compile` (sem args, modo interativo)

### Grammar

```text
tex-cli compile
```

### Behavior

1. Carrega config (exit 10/11).
2. Guard TTY via `std::io::IsTerminal`:
   - Não-TTY → exit `1` com stderr `Menu interativo de compile
     requer terminal. Use tex-cli compile <tex-file>.`.
3. Prompts sequenciais:
   - `inquire::Text` para "Caminho do `.tex` a compilar:".
   - `inquire::Confirm` "Manter cópia do .tex no output_dir?"
     (default = `config.compiler.keep_tex`).
   - `inquire::Confirm` "Manter cópia do .log no output_dir?"
     (default = `config.compiler.keep_logs`).
4. Constrói `CompileArgs { tex_file: Some(path), output: None,
   engine: None, keep_tex: <confirm>, no_keep_tex: !<confirm>,
   keep_logs: <confirm>, no_keep_logs: !<confirm>, force: false }`
   e chama `handle_compile(args)`.
5. Exit code = exit do `handle_compile` resultante.
6. Ctrl+C em qualquer prompt → exit `15`.

### Exit codes

Mesmos do subcomando direto + `1` para "menu requer TTY".

### Contratos observáveis

- Sem TTY: retorna `1` sem tentar prompt.
- TTY com path inexistente: cai no fluxo direto → exit `1`
  (I/O NotFound).
- TTY com path válido e tectonic disponível: PDF gerado.

---

## Ausências deliberadas (fora do escopo v1)

- Sem `--timeout <sec>` — v1 assume single-user, sem processos
  pendurados. Se surgir, spec 006 futura.
- Sem `--watch` — rebuild automático em file change. Fora do
  escopo pessoal (usa entr/watchexec).
- Sem passthrough de flags do engine (`--engine-args "..."`).
- Sem `--dry-run` — compile é lento e não faz sentido dry-runar.
- Sem múltiplas passes automáticas de bibtex fora do latexmk.
- Sem output JSON estruturado do resultado (fica pra observabilidade
  futura).
- Sem suporte a `.tex` que faz `\input{outra-parte.tex}` fora do
  dir do fonte (assumption explícita da spec).
