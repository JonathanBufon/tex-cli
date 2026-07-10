# Feature Specification: Renderização JSON → `.tex` via Tera

**Feature Branch**: `003-render-tera`

**Created**: 2026-07-09

**Status**: Draft

**Input**: User description: "Renderização JSON → `.tex` via `tera`. Subcomando `tex-cli render <template> <data.json>` + modo interativo. Cerne do pipeline Data/Template/PDF — materializa a separação declarada no Princípio II da constitution."

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Renderizar template com JSON válido para arquivo (Priority: P1) 🎯 MVP

O dev tem um template registrado (`artigo.tex` no `paths.templates_dir`)
e um arquivo JSON (`dados.json`) com as variáveis que o template
espera. Quer gerar o `.tex` intermediário populado com esses dados,
pronto pra ser compilado pra PDF numa etapa futura.

**Why this priority**: Sem este comando, `templates` da spec 002 fica
órfão — o usuário tem templates mas não consegue fazer nada com eles.
É a peça central do pipeline JSON → LaTeX que dá razão de existir ao
projeto (Princípio II da constitution).

**Independent Test**: Fixture com template `artigo.tex` contendo
`{{ titulo }}` + fixture com `dados.json` contendo
`{"titulo": "Meu Artigo"}` → rodar `tex-cli render artigo dados.json`
→ validar que existe arquivo `.tex` no `output_dir` cujo conteúdo tem
literalmente "Meu Artigo" no lugar do placeholder.

**Acceptance Scenarios**:

1. **Given** um template `artigo.tex` com placeholder `{{ titulo }}`
   e um JSON `{"titulo": "T"}`, **When** rodo `tex-cli render
   artigo dados.json`, **Then** o arquivo
   `<output_dir>/artigo.tex` é criado com o `T` substituído no
   lugar de `{{ titulo }}`, exit `0`, stdout contém `Renderizado em
   <caminho-absoluto>`.
2. **Given** o mesmo cenário mas com `--output /tmp/custom.tex`,
   **When** rodo o comando, **Then** o `.tex` é gravado em
   `/tmp/custom.tex` e não em `<output_dir>/artigo.tex`.
3. **Given** template com variável `{{ ausente }}` e JSON sem essa
   chave, **When** rodo o comando, **Then** exit code é dedicado a
   tera-error (proposta 30), stderr mostra nome do template + posição
   (linha/coluna) do erro + mensagem humana em pt-BR, e nenhum
   arquivo de saída é criado (atomicidade).
4. **Given** template válido e JSON malformado (falta `}` ou similar),
   **When** rodo o comando, **Then** exit code é dedicado a
   json-error (proposta 31), stderr localiza o erro (linha/coluna do
   `serde_json`), sem arquivo criado.
5. **Given** o output já existe no destino e sem `--force`, **When**
   rodo em ambiente não-TTY, **Then** exit `15` (user-aborted), o
   arquivo original permanece byte-a-byte igual.
6. **Given** o output já existe, com `--force`, **When** rodo,
   **Then** arquivo é sobrescrito atomicamente, stdout menciona
   "sobrescrito".

---

### User Story 2 - Iterar rapidamente com `--dry-run` (Priority: P2)

O dev está afinando um template e quer ver como o `.tex` renderizado
vai ficar sem poluir o `output_dir` a cada tentativa.

**Why this priority**: Loop de desenvolvimento fica muito mais rápido
sem I/O de gravação e cleanup. Não é MVP porque `render` sem
`--dry-run` já produz output usável (o dev pode simplesmente
inspecionar o arquivo com editor).

**Independent Test**: Rodar `tex-cli render artigo dados.json
--dry-run` → validar que stdout tem o `.tex` renderizado (byte-a-byte
igual ao que seria gravado no arquivo), stderr apenas com banner e
warnings, exit 0, **nenhum arquivo** criado no `output_dir`.

**Acceptance Scenarios**:

1. **Given** template + JSON válidos, **When** rodo `render ...
   --dry-run`, **Then** stdout recebe o `.tex` renderizado, exit 0,
   nenhum arquivo no `output_dir`.
2. **Given** o mesmo cenário, **When** o comando é encadeado com
   pipe (`... --dry-run | wc -l`), **Then** o pipe funciona (stdout
   é limpo, sem interferência de logs) e o receiver processa o
   conteúdo normalmente.
3. **Given** template com erro do tera + `--dry-run`, **When** rodo,
   **Then** exit code de tera-error (30) — mesmo comportamento do
   caminho normal; `--dry-run` não suprime erros, só suprime o I/O
   de gravação.

---

### User Story 3 - Ler JSON de stdin (Priority: P2)

O dev tem um pipeline shell/CI onde o JSON é gerado dinamicamente
(por exemplo, resultado de uma query ao banco ou API) e vai
diretamente para `render` sem passar por arquivo intermediário.

**Why this priority**: Composição Unix é obrigação básica pra
ferramenta CLI decente. Substituível por `render tpl $(mktemp)` mas
custa duas linhas extras no script.

**Independent Test**: `echo '{"titulo":"X"}' | tex-cli render artigo -`
→ mesmo output que gravar em `dados.json` e passar como arg.

**Acceptance Scenarios**:

1. **Given** template válido e JSON piped via stdin, **When** rodo
   `tex-cli render artigo -`, **Then** stdin é consumido, JSON
   parseado, template renderizado, `.tex` gravado normalmente no
   `output_dir` (ou stdout se `--dry-run`).
2. **Given** stdin vazio ou não-JSON, **When** rodo com arg `-`,
   **Then** exit code de json-error (31), stderr identifica origem
   como "stdin".

---

### User Story 4 - Modo interativo `render` (Priority: P3)

Um dev novo (ou casual) roda `tex-cli render` sem args e é guiado
pela ferramenta.

**Why this priority**: Segunda metade do Princípio III (interface
dupla). Não é MVP porque os subcomandos diretos já cobrem o
essencial pra dev experiente.

**Independent Test**: Rodar em TTY, escolher template do Select,
digitar path do JSON, confirmar output/dry-run → mesmo resultado que
os args diretos.

**Acceptance Scenarios**:

1. **Given** terminal interativo com templates registrados, **When**
   rodo `tex-cli render` sem args, **Then** aparece Select com
   nomes dos templates disponíveis (populado por `list_templates` da
   spec 002), depois Text pro path do JSON, depois Confirm pra
   `--dry-run` (default: não).
2. **Given** mesmo cenário mas sem TTY (script CI), **When** rodo,
   **Then** exit code 1 com stderr `Menu interativo de render
   requer terminal. Use tex-cli render <template> <data.json>`.
3. **Given** nenhum template registrado, **When** entro no modo
   interativo, **Then** stderr informa `Nenhum template encontrado
   em <path>. Adicione um com tex-cli templates add.` e exit 22.

---

### Edge Cases

- **JSON com tipos aninhados** (arrays/objects): tera suporta acesso
  via `{{ obj.chave }}` e `{{ array[0] }}` — funciona como esperado.
  Não validamos explicitamente, tera propaga erro se acesso inválido.
- **Template sem placeholders** (`.tex` estático): renderiza como
  cópia literal do template, exit 0. Comportamento "no-op-útil".
- **Placeholder com filtro** (`{{ nome | upper }}`): funciona porque
  usamos o registry padrão do tera (upper/lower/default/etc.).
- **Template com `\input{...}` LaTeX**: renderizado como texto —
  o `\input` só é resolvido em compilação (spec 004). Documentar
  claramente que este comando não segue includes.
- **Output path aponta pra diretório inexistente**: cria dirs pais
  via `create_dir_all` antes do write atômico. Comportamento
  simétrico ao `Config::save_atomic` da spec 001.
- **JSON top-level array em vez de objeto**: rejeitado com erro
  específico (proposta: exit 31 mesmo, mensagem "esperado objeto,
  recebido array") — tera não sabe usar array como contexto raiz.
- **JSON com chave contendo caractere reservado do tera** (ex.:
  `.` ou `-`): tera trata `.` como acesso a subcampo — chaves com
  pontos precisam ser reescritas pelo usuário. Documentado nas
  assumptions.
- **Template extremamente grande** (>1 MB): sem tratamento especial,
  tera carrega em memória. Fora do caso de uso comum.
- **Loop de render** (dois processos concorrentes escrevendo mesmo
  path): sem lock — assumimos uso single-process (herdado da
  constitution).
- **`--dry-run` combinado com `--output`**: `--dry-run` prevalece,
  ignora `--output` silenciosamente. Documentado.
- **Path do output contém `~` ou é relativo**: expandido via
  `paths::expand_user_path` (spec 001) antes da gravação.

## Requirements *(mandatory)*

### Functional Requirements

**Comando `render` — args positionais e core (FR-001..FR-006)**:

- **FR-001**: O sistema MUST expor `tex-cli render <template-name>
  <data-source>` onde `<template-name>` é resolvido via
  `resolve_template` da spec 002 (aceita nome com ou sem `.tex`).
- **FR-002**: `<data-source>` MUST ser o caminho de um arquivo JSON
  no filesystem OU o literal `-` para indicar "leia de stdin".
- **FR-003**: O JSON MUST ser um objeto top-level (não array, não
  literal). Deserializado como `serde_json::Value` e convertido em
  contexto tera via `tera::Context::from_value`.
- **FR-004**: O template MUST ser lido via `read_template` (spec 002)
  e renderizado via `tera::Tera::one_off(&template_src, &context,
  autoescape=false)` — templates LaTeX não devem sofrer escape HTML.
- **FR-005**: O output rendered MUST ser gravado atomicamente
  (tmpfile + persist) com permissão `0644`, mesmo padrão do
  `templates add` da spec 002.
- **FR-006**: Path default do output MUST ser
  `<config.paths.output_dir>/<template-name>.tex`. Pais criados
  automaticamente se não existirem.

**Flags (FR-007..FR-010)**:

- **FR-007**: `--output <path>` (ou `-o`) MUST substituir o path
  default, aceitando `~` e caminhos relativos (expandidos via
  `paths::expand_user_path` da spec 001).
- **FR-008**: `--dry-run` MUST imprimir o `.tex` renderizado em
  stdout e **não** gravar arquivo algum. Quando combinado com
  `--output`, `--output` é silenciosamente ignorado.
- **FR-009**: `--force` MUST sobrescrever arquivo existente no
  destino sem prompt. Sem `--force` e com arquivo pré-existente,
  comportamento seguindo padrão da spec 002: TTY → prompt de
  confirmação; não-TTY → exit 15.
- **FR-010**: A flag global `-v/--verbose` MUST continuar
  funcionando (herdado das specs 001/002).

**Modo interativo (FR-011..FR-013)**:

- **FR-011**: `tex-cli render` sem args positionais MUST abrir menu
  inquire em TTY: Select com nomes de templates (populado por
  `list_templates` do `paths.templates_dir`), depois Text pro path
  do JSON, depois Confirm pra `--dry-run` (default: não).
- **FR-012**: Em ambiente não-TTY, `tex-cli render` sem args MUST
  retornar exit `1` com stderr orientando o uso de subcomando
  direto.
- **FR-013**: Se o diretório de templates está vazio, o menu MUST
  falhar cedo com stderr orientando `tex-cli templates add`, sem
  abrir Select vazio.

**Erros e códigos (FR-014..FR-019)**:

- **FR-014**: Erros do tera (variável indefinida, syntax error,
  filtro inexistente, tipo incompatível) MUST retornar exit code
  dedicado (proposto **30** — TeraRenderError) com stderr
  identificando o template, a linha/coluna do erro (via
  `tera::Error::source`) e mensagem humana em pt-BR.
- **FR-015**: JSON malformado (parse fail) OU JSON com forma inválida
  (não-objeto no top-level) MUST retornar exit code dedicado
  (proposto **31** — InvalidJson) com stderr localizando o erro
  (linha/coluna via `serde_json::Error`) OU descrevendo a forma
  esperada.
- **FR-016**: Config ausente/corrompido MUST retornar exit `10`/`11`
  (herdado spec 001).
- **FR-017**: Template não encontrado MUST retornar exit `20`
  (herdado spec 002).
- **FR-018**: `paths.output_dir` inexistente é permitido — o sistema
  MUST criar dirs pais durante o write. Se `paths.templates_dir`
  inexistente (necessário pra resolver template), exit `22`
  (herdado spec 002).
- **FR-019**: Permissão negada em qualquer I/O MUST retornar exit
  `14` (herdado spec 001).

**Contratos observáveis (FR-020..FR-023)**:

- **FR-020**: Banner "TEX CLI" MUST continuar em stderr e nunca em
  stdout (herdado spec 001), garantindo que `render --dry-run` seja
  pipeable sem contaminação.
- **FR-021**: Em `render --dry-run`, o stdout MUST conter
  **exclusivamente** o `.tex` renderizado — nem prefixo, nem
  cabeçalho, nem newline extra.
- **FR-022**: Em `render` normal (grava arquivo), stdout MUST ter
  exatamente uma linha `Renderizado em <path-absoluto>` (ou
  `Renderizado (sobrescrito) em <path>` quando aplicável).
- **FR-023**: Nenhum arquivo parcial MUST permanecer no destino em
  caso de erro pós-renderização (garantido por escrita atômica).

### Key Entities

- **RenderContext** (conceitual, não persistido): a struct que
  agrega o `tera::Context` construído a partir do JSON. Recomputada
  em cada invocação. Sem estado entre execuções.
- **RenderOutcome** (retornado por `render_and_write`): struct com
  `output_path: PathBuf`, `bytes_written: u64`,
  `overwrote_existing: bool`, `dry_run: bool`. Consumido apenas pelo
  handler pra montar a mensagem final.
- **Extensão de `TexError`**: duas variantes novas —
  `TeraRenderError { template_name, detail }` → exit `30`, e
  `InvalidJson { source: String, detail }` → exit `31`. Não persiste;
  vive só na fronteira de erro.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Um dev consegue renderizar um template de 5-10 KB com
  10 variáveis em **< 100 ms** wall-clock. Justificativa: tera é
  puramente in-memory, contexto pequeno, o dominante seria I/O de
  disco (spec 002 provou < 5 ms pra arquivos dessa ordem).
- **SC-002**: Um script CI/automação consegue: (a) piped JSON via
  stdin, (b) renderizar num path custom via `--output`, (c) verificar
  presença do arquivo — **sem intervenção interativa** em nenhum
  passo.
- **SC-003**: Uma mensagem de erro de tera (variável indefinida)
  **inclui linha e coluna** do template em > 90% dos casos (tera
  fornece essa info na maioria das falhas; alguns filtros custom
  podem não — mas usamos só filtros default, então taxa efetiva
  esperada = 100%).
- **SC-004**: Um dev que nunca usou tera consegue descobrir a sintaxe
  básica (`{{ var }}`, `{% if %}`, filtro `default`) em **< 5
  minutos** lendo o `--help` do subcomando + os exemplos em
  `examples/templates/`.
- **SC-005**: `render --dry-run` produz output **byte-a-byte
  idêntico** ao arquivo que seria gravado sem `--dry-run` (validado
  por teste que compara stdout do dry-run com o `read` do arquivo
  gravado sem dry-run).
- **SC-006**: `render` interrompido no meio (SIGKILL) **não deixa**
  arquivo parcial no destino (validado por teste conceitual de
  atomicidade — o mesmo padrão da spec 002).

## Assumptions

- **Sintaxe do template**: assume-se sintaxe padrão do tera —
  `{{ var }}`, `{% if cond %}`, `{% for item in list %}`, filtros
  built-in (`upper`, `lower`, `default`, `join`, `length`, etc.). Não
  registramos filtros customizados na v1.
- **Forma do JSON**: assume-se objeto top-level (`{...}`). Arrays no
  top-level (`[...]`) são rejeitados com mensagem clara — tera não
  aceita array como contexto raiz.
- **Escape**: renderização usa `autoescape=false` porque `.tex` não
  requer escape HTML (o que o tera faz por default para `.html`).
  Usuário responsável por escapar caracteres especiais do LaTeX
  (`&`, `%`, `$`, `_`, `#`) diretamente no template ou via filtro
  customizado futuro — fora do escopo v1.
- **Chaves com `.` ou `-`**: nomes de chave JSON contendo `.` são
  interpretados pelo tera como acesso a subcampo. Chaves com `-` são
  rejeitadas pelo parser do tera. Usuário deve nomear campos com
  underscore/camelCase — documentado como known limitation.
- **Contexto imutável**: o mesmo JSON produz sempre o mesmo `.tex`
  (dado template idêntico). Sem randomness, sem timestamps
  injetados, sem env vars.
- **Follow-up SIGPIPE da spec 002**: continua aplicável (o `--dry-run`
  pipeado a `head` pode panicar). Fix registrado no follow-up da
  spec 002; não bloqueia esta spec.
- Templates que fazem `\input{outra-parte.tex}` do LaTeX são
  renderizados como texto — o `\input` só é resolvido pelo
  compilador (spec 004).

## Dependencies

- **Spec 001 (config-init-management)** MUST estar mergeada. Reusa:
  - `Config::load` para ler `paths.output_dir` e `paths.templates_dir`.
  - `paths::expand_user_path` para normalizar `--output`.
  - Exit codes 10/11/14/15.
- **Spec 002 (templates-management)** MUST estar mergeada. Reusa:
  - `resolve_template` para localizar o template pelo nome.
  - `read_template` para carregar bytes.
  - Padrão de escrita atômica com `0o644`.
  - Exit codes 20/22.
- **Nova dependência**: o crate `tera` da stack canônica da
  constitution (já listado nas Restrições Técnicas). Nenhuma dep
  adicional necessária — `tera` é o único crate novo a ser
  ativado no `Cargo.toml`.
