# Feature Specification: Gestão de Templates LaTeX (list/show/add/remove)

**Feature Branch**: `002-templates-management`

**Created**: 2026-07-09

**Status**: Draft

**Input**: User description: "Gestão de templates LaTeX reaproveitáveis dentro do `paths.templates_dir` configurado na spec 001. Quatro subcomandos (`list`, `show`, `add`, `remove`) + modo interativo. Herda banner, `-v`, códigos de saída e escrita atômica onde aplicável da spec 001."

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Listar templates disponíveis (Priority: P1) 🎯 MVP

Um usuário do Tex já rodou `tex-cli init` e mantém alguns templates
`.tex` no diretório configurado. Quer ver rapidamente o que tem
disponível sem precisar sair do terminal ou decorar nomes.

**Why this priority**: Sem listagem, o usuário depende de memória ou de
sair do fluxo pra abrir um gerenciador de arquivos. É a operação mais
frequente e a que desbloqueia todas as outras (você precisa saber o
nome antes de `show`/`remove`).

**Independent Test**: Pode ser totalmente exercitado por: (1) criar
alguns arquivos `.tex` no `paths.templates_dir` do config, (2) rodar
`tex-cli templates list`, (3) validar que a saída contém nome, tamanho
e data de modificação de cada template. Também `--format=json` produz
array parseável.

**Acceptance Scenarios**:

1. **Given** o config aponta pra um diretório com 3 arquivos `.tex`,
   **When** rodo `tex-cli templates list`, **Then** stdout mostra 3
   linhas, uma por template, com nome (sem `.tex`), tamanho em bytes
   e data de modificação legível.
2. **Given** o mesmo cenário, **When** rodo `tex-cli templates list
   --format=json`, **Then** stdout é um JSON array com 3 objetos, cada
   um com campos `name`, `path`, `size_bytes`, `modified_at`
   (RFC3339).
3. **Given** o `paths.templates_dir` está vazio, **When** rodo
   `templates list`, **Then** stdout mostra `Nenhum template
   encontrado em <path>.` e exit `0`.
4. **Given** o `paths.templates_dir` contém arquivos não-`.tex`
   (ex.: `.bib`, `.png`, `.aux`), **When** rodo `templates list`,
   **Then** apenas os `.tex` aparecem — outros são silenciosamente
   ignorados.

---

### User Story 2 - Inspecionar conteúdo de um template (Priority: P2)

O usuário lembra que tem um template chamado "artigo" mas não sabe
mais o que ele contém — quer ver antes de decidir usar (ou antes de
modificar).

**Why this priority**: Complementa listagem. Sem ela, o usuário
recorre a `cat`/`less`, quebrando o fluxo dentro da CLI. Não é MVP
porque `cat` é aceitável como fallback temporário.

**Independent Test**: Com um template pré-criado (fixture), rodar
`tex-cli templates show artigo` e validar que stdout é
byte-por-byte igual ao conteúdo do arquivo. Também testar exit code
específico quando o nome não existe.

**Acceptance Scenarios**:

1. **Given** existe `artigo.tex` no diretório, **When** rodo
   `tex-cli templates show artigo`, **Then** stdout contém o
   conteúdo bruto do arquivo, sem prefixo/cabeçalho.
2. **Given** existe `artigo.tex`, **When** rodo `tex-cli templates
   show artigo.tex` (com extensão), **Then** o comportamento é
   idêntico — a extensão é aceita mas opcional.
3. **Given** não existe `parecer.tex`, **When** rodo `tex-cli
   templates show parecer`, **Then** exit code é um valor específico
   (novo, distinto de config-missing) e stderr traz mensagem `Template
   'parecer' não existe em <path>.`.

---

### User Story 3 - Adicionar novo template ao repositório (Priority: P3)

O usuário produziu um novo `.tex` para um caso específico
(`relatorio_2026.tex`) e quer promovê-lo ao diretório central de
templates para reusar depois.

**Why this priority**: Substituível por `cp` do sistema, mas
adicionar uma etapa dentro da CLI garante confirmação de overwrite,
validação de que o arquivo é texto UTF-8 e permite o modo interativo
guiar o processo.

**Independent Test**: Criar arquivo `.tex` de teste no host, rodar
`tex-cli templates add /caminho/relatorio_2026.tex`, validar que
existe no diretório de templates e o conteúdo bate. Testar também
`--name` (renomeando) e confirmação de overwrite.

**Acceptance Scenarios**:

1. **Given** existe `/tmp/novo.tex` no host e o diretório de
   templates não tem `novo.tex`, **When** rodo `tex-cli templates
   add /tmp/novo.tex`, **Then** o arquivo aparece em
   `<templates_dir>/novo.tex` com conteúdo idêntico, e stdout mostra
   `Template 'novo' adicionado em <caminho-completo>.`.
2. **Given** o mesmo cenário, **When** rodo `tex-cli templates add
   /tmp/novo.tex --name artigo`, **Then** o arquivo é salvo como
   `<templates_dir>/artigo.tex`.
3. **Given** já existe `artigo.tex` no diretório de templates,
   **When** rodo `tex-cli templates add /tmp/artigo.tex` sem
   `--force`, **Then** o comando pede confirmação (modo interativo)
   ou aborta com exit code específico (modo não-interativo — nenhuma
   TTY) — em nenhum caso sobrescreve silenciosamente.
4. **Given** `/tmp/binario.tex` contém bytes não-UTF8, **When** rodo
   `tex-cli templates add /tmp/binario.tex`, **Then** o comando
   aborta com exit code específico e stderr `Arquivo '/tmp/binario.tex'
   não é texto UTF-8 válido.`.
5. **Given** o `paths.templates_dir` não existe, **When** rodo
   `tex-cli templates add ...`, **Then** exit code é o mesmo de
   config-missing (10) OU um código dedicado, e stderr orienta o
   usuário a rodar `tex-cli init` ou verificar `config show`.

---

### User Story 4 - Remover template obsoleto (Priority: P3)

O usuário tem um template antigo (`carta_v1.tex`) que virou legado
depois de refazer o layout. Quer removê-lo pra evitar confusão.

**Why this priority**: Substituível por `rm`, mas remoção via CLI
protege contra typo (via confirmação obrigatória) e integra com o
menu interativo.

**Independent Test**: Fixture com template pré-existente, rodar
`tex-cli templates remove <nome>` respondendo "não" à confirmação
→ arquivo persiste. Rodar de novo com `--force` → arquivo removido.

**Acceptance Scenarios**:

1. **Given** existe `carta.tex` no diretório, **When** rodo
   `tex-cli templates remove carta` sem `--force`, **Then** o
   comando pede confirmação interativa (default = "não") ou, em
   ambiente não-interativo, aborta com exit code de user-aborted (15)
   sem tocar no arquivo.
2. **Given** o mesmo cenário, **When** rodo `tex-cli templates
   remove carta --force`, **Then** o arquivo é removido e stdout
   mostra `Template 'carta' removido de <caminho>.`.
3. **Given** não existe `carta.tex`, **When** rodo `tex-cli
   templates remove carta`, **Then** exit code é o mesmo de
   template-ausente (novo) e stderr traz `Template 'carta' não
   existe em <path>.`.

---

### User Story 5 - Modo interativo `templates` (Priority: P3)

Um usuário novo (ou casual) executa `tex-cli templates` sem
subcomando e espera ser guiado por menu.

**Why this priority**: Cumpre a segunda metade do Princípio III da
constitution (interface dupla). Sem isso, o modo interativo do
comando `templates` fica órfão. Não é MVP porque os subcomandos
diretos já cobrem o essencial.

**Independent Test**: Rodar `tex-cli templates` em TTY, navegar pelo
menu com setas, escolher "listar" e ver o resultado. Fora do escopo
de testes automatizados (menu inquire exige TTY, coberto por
smoke manual no quickstart).

**Acceptance Scenarios**:

1. **Given** um terminal interativo, **When** rodo `tex-cli
   templates`, **Then** aparece um menu com 5 opções: "Listar
   templates", "Inspecionar template", "Adicionar template",
   "Remover template", "Sair".
2. **Given** o mesmo terminal, **When** escolho "Sair" ou pressiono
   Ctrl+C, **Then** o comando retorna exit `0` (sair) ou `15`
   (Ctrl+C) sem erro.

---

### Edge Cases

- **`paths.templates_dir` é um symlink**: o comando segue o symlink e
  opera no diretório final. Sem tratamento especial.
- **Template com nome contendo espaços ou caracteres especiais**:
  suportado — o nome é o basename sem `.tex`; usuário pode precisar
  citar em shell.
- **Dois arquivos com case diferente** (`artigo.tex` e
  `Artigo.tex`): tratados como templates distintos no listar, mas
  `show artigo` retorna o primeiro que casar case-sensitive; se
  ambíguo, retorna erro pedindo o nome exato com extensão.
- **Diretório contém subdiretórios**: subdiretórios são ignorados na
  v1 — só arquivos `.tex` no nível raiz do `templates_dir` contam.
- **Template é link simbólico**: `list` mostra normalmente; `show`
  segue o link; `remove` remove o link, não o alvo.
- **`add` sobre arquivo do host que é symlink**: copia o conteúdo
  final (segue link), não copia o link.
- **`paths.templates_dir` sem permissão de leitura**: exit code 14
  (PermissionDenied) reaproveitando o da spec 001.
- **Concorrência**: dois processos `tex-cli templates` rodando ao
  mesmo tempo não têm garantia de consistência; assumimos uso
  single-user single-process (herda da constitution).

## Requirements *(mandatory)*

### Functional Requirements

**Listagem (FR-001..FR-004)**:

- **FR-001**: O sistema MUST expor `tex-cli templates list` que
  enumera todos os arquivos com extensão `.tex` no diretório apontado
  por `config.paths.templates_dir`.
- **FR-002**: O sistema MUST suportar `--format=<humano|json>` no
  `templates list`, com `humano` (default) como listagem tabular
  legível e `json` como array de objetos parseáveis por `jq`.
- **FR-003**: Cada entrada da listagem MUST expor: nome canônico do
  template (basename sem `.tex`), path absoluto, tamanho em bytes,
  data da última modificação (formato ISO 8601 / RFC3339 no JSON,
  formato humano-legível no `humano`).
- **FR-004**: O sistema MUST retornar exit `0` mesmo quando o
  diretório está vazio; a saída deve conter mensagem informativa em
  vez de erro.

**Inspeção (FR-005..FR-007)**:

- **FR-005**: O sistema MUST expor `tex-cli templates show <nome>`
  que imprime em stdout o conteúdo bruto do template correspondente.
- **FR-006**: `<nome>` MUST aceitar tanto `artigo` quanto
  `artigo.tex` — internamente ambos resolvem para o mesmo arquivo.
- **FR-007**: Se `<nome>` não corresponder a nenhum arquivo, o
  sistema MUST emitir mensagem humana em stderr identificando o
  template solicitado e o diretório inspecionado, e retornar exit
  code dedicado a template-ausente (novo — proposto 20).

**Adição (FR-008..FR-013)**:

- **FR-008**: O sistema MUST expor `tex-cli templates add <arquivo>
  [--name <nome>] [--force]` que copia o `<arquivo>` do host para o
  diretório de templates.
- **FR-009**: O nome do template resultante MUST ser: (a) `<nome>` se
  `--name` fornecido, (b) basename sem `.tex` do `<arquivo>` caso
  contrário.
- **FR-010**: Antes de gravar, o sistema MUST verificar que o
  conteúdo do arquivo é UTF-8 válido; caso contrário, aborta com
  exit code dedicado (proposto 21) e mensagem clara.
- **FR-011**: Se já existe template com o mesmo nome no destino, o
  sistema MUST pedir confirmação (`inquire::Confirm` default "não")
  quando executado em TTY; sem `--force` e sem TTY, retorna exit
  code de user-aborted (15).
- **FR-012**: A gravação MUST ser atômica (tmpfile + rename) para
  não deixar arquivo pela metade em caso de falha durante a cópia.
- **FR-013**: O arquivo gravado MUST ter permissão padrão do umask do
  usuário (tipicamente `0644`), **não** `0600` — justificativa: o
  usuário pode versionar/publicar templates via git, ao contrário
  do config.

**Remoção (FR-014..FR-016)**:

- **FR-014**: O sistema MUST expor `tex-cli templates remove <nome>
  [--force]`.
- **FR-015**: Sem `--force`, o sistema MUST pedir confirmação
  interativa (`inquire::Confirm` default "não"); em ambiente
  não-interativo sem `--force`, retorna exit `15` (user-aborted) sem
  remover.
- **FR-016**: Se `<nome>` não existe, retorna o mesmo exit code de
  template-ausente (20 proposto) que o `show`.

**Modo interativo (FR-017..FR-018)**:

- **FR-017**: O sistema MUST expor `tex-cli templates` (sem
  subcomando) que abre um menu `inquire::Select` com 5 opções:
  "Listar templates", "Inspecionar template", "Adicionar template",
  "Remover template", "Sair".
- **FR-018**: Cada opção do menu MUST executar o mesmo caminho de
  código dos subcomandos diretos correspondentes, prompteando por
  parâmetros adicionais quando necessário (ex.: nome do template
  para "Inspecionar").

**Transversal — herdado / reaproveitado da spec 001 (FR-019..FR-023)**:

- **FR-019**: Todos os subcomandos MUST emitir o banner "TEX CLI" em
  stderr antes de qualquer saída útil, exatamente como declarado no
  contract da spec 001 (nunca em stdout).
- **FR-020**: A flag global `-v/--verbose` MUST continuar funcionando
  em todos os subcomandos deste grupo (WARN/INFO/DEBUG/TRACE).
- **FR-021**: Se `~/.config/tex/config.toml` está ausente, todos os
  subcomandos deste grupo MUST retornar exit `10` com mensagem
  orientando `tex-cli init`.
- **FR-022**: Se o config está corrompido, exit `11` — mesma
  semântica da spec 001.
- **FR-023**: Se o diretório `paths.templates_dir` não existe ou não
  é acessível para leitura, o sistema MUST retornar exit `14`
  (PermissionDenied já existente) para leitura sem permissão, ou um
  novo exit code (proposto 22) para diretório inexistente,
  orientando o usuário a corrigir via `config set` ou `mkdir`.

### Key Entities

- **Template**: um arquivo `.tex` residindo no nível raiz de
  `paths.templates_dir`. Atributos: nome canônico (basename sem
  extensão), path absoluto, tamanho em bytes, timestamp de última
  modificação. Não tem estado além do próprio arquivo — sem metadata
  side-car.
- **TemplatesRegistry** (conceitual, sem persistência dedicada): a
  visão que a CLI tem do conjunto de templates disponíveis em um
  instante do tempo. Recomputada em cada invocação a partir de
  `std::fs::read_dir` do `paths.templates_dir`. Não há cache.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Um usuário consegue descobrir os nomes exatos dos
  templates disponíveis em **< 5 segundos** a partir do momento em
  que digita o primeiro caractere de `tex-cli templates list`.
- **SC-002**: `tex-cli templates list --format=json` completa em
  **< 100 ms wall-clock** para diretórios com até 100 templates.
- **SC-003**: Um script CI/automação consegue adicionar um novo
  template, verificar sua presença via `list --format=json`, e
  removê-lo, sem intervenção interativa em nenhum passo (usando
  `--force` onde necessário).
- **SC-004**: Um novo usuário do Tex descobre e usa `templates
  show` em **< 30 segundos** após rodar `--help` do grupo
  `templates`.
- **SC-005**: Nenhum comando deste grupo deixa arquivo parcialmente
  gravado em caso de falha (verificado por teste que simula
  interrupção durante `add`).
- **SC-006**: A saída de `tex-cli templates list --format=json`
  passa em `jq empty` (é JSON válido) em **100% dos casos**, mesmo
  quando o diretório está vazio (retorna `[]`).

## Assumptions

- O usuário já rodou `tex-cli init` — a spec 001 é pré-requisito
  duro. Se o config está ausente, os subcomandos falham cedo com
  exit 10 orientando o setup.
- Todos os arquivos com extensão exatamente `.tex` (case-sensitive)
  no nível raiz de `paths.templates_dir` são considerados templates.
  Extensões alternativas (`.tex.j2`, `.template`, `.latex`) ficam
  fora nesta v1.
- O nome do template é único no diretório. Se o usuário criar dois
  arquivos com case diferente (`artigo.tex` e `Artigo.tex`), a v1
  trata como distintos mas `show <nome>` faz busca case-sensitive e
  retorna erro de ambiguidade se necessário.
- Templates não têm assets associados (imagens, `.bib`, etc.) nesta
  v1 — o comando `add` copia apenas o arquivo `.tex`.
- O usuário aceita que a permissão dos templates seja `0644`
  (padrão) em vez de `0600` para poder versionar em git.
- Subdiretórios dentro de `paths.templates_dir` são ignorados. Se o
  usuário quiser hierarquizar, isso será feature de spec futura
  (talvez `templates/categorias` ou tags — fora do escopo agora).
- O comando não valida sintaxe LaTeX — apenas que o arquivo é texto
  UTF-8. Um `.tex` malformado passará por `add` sem erro.

## Dependencies

- **Spec 001 (config-init-management)** MUST estar mergeada.
  Especificamente:
  - `Config::load()` deve estar disponível pra ler
    `paths.templates_dir`.
  - `TexError::ConfigMissing` e códigos 10/11/14/15 são reaproveitados.
  - Banner asset e `include_str!` continuam iguais.
  - Estrutura de módulos: novo arquivo `src/templates.rs` já
    previsto pela constitution ("Estrutura interna esperada").
