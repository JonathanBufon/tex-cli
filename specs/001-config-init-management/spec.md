# Feature Specification: Configuração Inicial e Gestão do Config do Tex

**Feature Branch**: `001-config-init-management`

**Created**: 2026-07-09

**Status**: Draft

**Input**: User description: "Configuração inicial e gestão do config do Tex CLI — `init`, `config show`, `config set` — arquivo TOML em `~/.config/tex/config.toml`."

## Clarifications

### Session 2026-07-09

- Q: Formato das chaves aceitas por `config set` (flat, dotted, ou misto)? → A: Sempre dotted path — uma única forma canônica (`paths.templates_dir`, `paths.output_dir`, `compiler.engine`, `compiler.keep_tex`, `compiler.keep_logs`, `behavior.ask_output_path_every_time`). Sem aliases.
- Q: Escrita do arquivo de config deve ser atômica em caso de crash mid-write? → A: Sim, sempre — via arquivo temporário no mesmo diretório e rename atômico. Não aceitamos janela de corrupção.
- Q: `config show` precisa de formato estruturado para scripts, ou só humano? → A: Padrão humano legível; suporta flag `--format=<humano|json|toml>` para consumo por scripts (json/toml determinísticos, sem cores nem enfeites).
- Q: Que permissão UNIX o arquivo de config deve receber ao ser criado? → A: `0600` (leitura e escrita apenas para o dono), consistente com `gh`, `aws`, `ssh` — defesa em profundidade mesmo sem conteúdo secreto.
- Q: Como o usuário controla a verbosidade de logs desta feature? → A: Silencioso por padrão; flag global `-v`/`--verbose` repetível (`-v` → info, `-vv` → debug, `-vvv` → trace); erros sempre em stderr independente do nível. Padrão Unix clássico.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Primeira configuração após instalar o Tex (Priority: P1)

Um usuário acaba de instalar o Tex no seu Linux e ainda não tem nenhum
config salvo. Ele quer preparar o ambiente uma única vez para depois
começar a gerar PDFs. Ele executa o comando de inicialização, responde
três perguntas guiadas (onde ficam os templates, para onde vão os PDFs,
qual compilador usar) e ao fim recebe a confirmação de que o config foi
gravado no local canônico do sistema.

**Why this priority**: Sem esta história, nenhuma outra parte do Tex
funciona — todo comando do MVP depende de saber onde buscar templates
e onde salvar PDFs. É o gate de entrada da ferramenta.

**Independent Test**: Em um sistema limpo (sem `~/.config/tex/`), rodar
o comando de inicialização, responder aos prompts, e verificar que o
arquivo TOML foi criado com as respostas dadas e com valores padrão
sensatos para os campos não perguntados.

**Acceptance Scenarios**:

1. **Given** o usuário não possui config salvo, **When** ele executa o
   comando de inicialização e responde os três prompts com valores
   válidos, **Then** o arquivo de config é criado no caminho canônico do
   sistema, contém as respostas do usuário, contém padrões sensatos para
   os campos não perguntados, e a ferramenta exibe uma mensagem de
   confirmação com o caminho do arquivo.
2. **Given** o usuário já possui um config salvo, **When** ele executa
   o comando de inicialização novamente, **Then** a ferramenta pergunta
   explicitamente se deve sobrescrever o config atual, e só prossegue
   após confirmação positiva.
3. **Given** o usuário informa um diretório de templates que não existe
   no disco, **When** o prompt é respondido, **Then** a ferramenta
   pergunta se deve criar esse diretório ou abortar, respeitando a
   escolha do usuário.
4. **Given** o usuário escolhe um compilador cujo binário não está
   instalado no sistema, **When** ele confirma a escolha, **Then** a
   ferramenta exibe um aviso claro de que o binário não foi encontrado,
   mas ainda assim salva a preferência no config para o usuário instalar
   depois.

---

### User Story 2 - Inspeção do config atual (Priority: P2)

Um usuário já configurado quer conferir rapidamente para onde os PDFs
estão indo, qual compilador está ativo, ou qual diretório de templates
está em uso — sem precisar abrir manualmente o arquivo TOML. Ele executa
um comando de inspeção e vê o config formatado de maneira legível no
terminal.

**Why this priority**: É uma necessidade recorrente durante o uso
normal (verificar antes de gerar um PDF importante) e é a maneira mais
segura de auditar o estado atual sem risco de modificar nada. Fica em
P2 porque não bloqueia o fluxo principal de conversão, mas é
essencialmente esperada em qualquer CLI de configuração.

**Independent Test**: Com um config já criado (por qualquer meio),
executar o comando de inspeção e verificar que a saída é humana
legível, corresponde ao conteúdo real do arquivo TOML e não modifica
o arquivo.

**Acceptance Scenarios**:

1. **Given** um config válido existe, **When** o usuário executa o
   comando de inspeção, **Then** a ferramenta exibe todos os campos
   do config agrupados por seção, em formato humano legível, e retorna
   com código de saída 0.
2. **Given** nenhum config existe ainda, **When** o usuário executa o
   comando de inspeção, **Then** a ferramenta exibe uma mensagem
   orientando a rodar o comando de inicialização primeiro, e retorna
   com código de saída diferente de 0.
3. **Given** um config existe mas está corrompido (TOML inválido),
   **When** o usuário executa o comando de inspeção, **Then** a
   ferramenta exibe o erro de parsing em linguagem humana (não stack
   trace bruto) e sugere rodar a inicialização novamente.

---

### User Story 3 - Alteração pontual de uma configuração (Priority: P3)

Um usuário quer trocar apenas o diretório padrão de saída de PDFs
(por exemplo, porque criou uma pasta nova para um novo projeto). Ele
não quer rodar toda a inicialização novamente nem editar o TOML à mão.
Ele passa a chave e o novo valor como argumentos diretos e a mudança é
aplicada.

**Why this priority**: Essencial para o modo automatizado/scriptável
exigido pela constituição (Princípio III). Fica em P3 porque o usuário
pode contornar editando o TOML manualmente ou rerodando a
inicialização, mas o comando torna a experiência muito melhor.

**Independent Test**: Com um config existente, executar o comando de
alteração passando uma chave e um valor válidos; verificar que apenas
aquela chave mudou no arquivo, que as demais chaves permanecem
idênticas e que nenhum prompt interativo foi disparado.

**Acceptance Scenarios**:

1. **Given** um config válido existe, **When** o usuário executa o
   comando de alteração passando uma chave suportada e um valor válido,
   **Then** apenas aquela chave é modificada no arquivo, as demais
   permanecem exatamente iguais, e a ferramenta confirma a mudança na
   saída.
2. **Given** um config válido existe, **When** o usuário passa uma
   chave que não é reconhecida, **Then** a ferramenta lista as chaves
   suportadas e retorna com erro sem tocar no arquivo.
3. **Given** o usuário altera um caminho de diretório para um destino
   que ainda não existe, **When** o comando é executado, **Then** a
   ferramenta salva a mudança mas exibe um aviso de que o diretório
   informado não existe no momento (sem abortar).
4. **Given** o usuário altera a chave do compilador para um valor que
   ainda não é suportado nesta versão, **When** o comando é executado,
   **Then** a ferramenta salva a mudança mas exibe um aviso de que
   aquele engine ainda não está implementado.

---

### Edge Cases

- O diretório pai de `~/.config/tex/` não existe (usuário nunca usou
  aplicativos que gravam ali). A ferramenta MUST criar a árvore
  necessária durante a inicialização.
- O usuário não tem permissão de escrita no caminho canônico do config
  (por exemplo, home read-only). A ferramenta MUST reportar erro claro
  indicando o caminho e a razão.
- O usuário passa um caminho relativo (`./templates`) no prompt ou no
  comando de alteração. A ferramenta MUST expandir para caminho
  absoluto antes de salvar, para que o config seja portável entre
  diretórios de trabalho.
- O usuário passa `~/algo` (com til) no prompt. A ferramenta MUST
  expandir o `~` para o home do usuário atual.
- O comando de alteração é executado sem config existente. A ferramenta
  MUST orientar rodar a inicialização primeiro, em vez de criar um
  config parcial.
- O usuário passa um valor não booleano para uma chave booleana
  (`compiler.keep_tex`, `compiler.keep_logs`,
  `behavior.ask_output_path_every_time`). A ferramenta MUST rejeitar
  com mensagem clara de quais valores são aceitos.
- Dois processos rodando `config set` simultaneamente. Fora do escopo
  da v1 — assume-se uso single-user, single-process.

## Requirements *(mandatory)*

### Functional Requirements

**Localização e formato do config**

- **FR-001**: O sistema MUST persistir o config em um arquivo texto no
  caminho canônico de configuração do usuário para o Tex, resolvido de
  forma consistente entre execuções.
- **FR-002**: O formato do arquivo MUST ser TOML, organizado em três
  seções lógicas: caminhos, compilador e comportamento.
- **FR-003**: O sistema MUST criar automaticamente qualquer diretório
  intermediário necessário para gravar o arquivo de config.
- **FR-003a**: Toda gravação do arquivo de config (seja pelo comando
  de inicialização ou pelo comando de alteração pontual) MUST ser
  **atômica**: o conteúdo é escrito primeiro em arquivo temporário no
  mesmo diretório do config final e então renomeado para o caminho de
  destino. Nunca ocorre gravação parcial visível ao usuário — em caso
  de crash durante a escrita, o arquivo anterior permanece íntegro.
- **FR-003b**: O arquivo de config, ao ser criado (ou reescrito via
  fluxo atômico), MUST receber as permissões UNIX **`0600`** — leitura
  e escrita apenas para o proprietário. Isto vale tanto para o path
  final quanto para o arquivo temporário usado no rename, evitando
  janela de visibilidade a outros usuários da máquina.

**Comando de inicialização**

- **FR-004**: O sistema MUST oferecer um comando dedicado que guie o
  usuário na criação inicial do config por meio de prompts sequenciais.
- **FR-005**: Os prompts do comando de inicialização MUST perguntar, no
  mínimo: diretório de templates, diretório padrão de saída e
  compilador preferido.
- **FR-006**: Se um config já existir, o comando de inicialização MUST
  exigir confirmação explícita antes de sobrescrevê-lo.
- **FR-007**: Se o diretório de templates informado não existir, o
  comando MUST perguntar se deve criá-lo ou abortar.
- **FR-008**: O comando MUST verificar se o binário do compilador
  escolhido está instalado no sistema; se ausente, exibir aviso claro
  mas ainda persistir a preferência.
- **FR-009**: Ao concluir, o comando MUST exibir o caminho do arquivo
  gravado.

**Comando de inspeção**

- **FR-010**: O sistema MUST oferecer um comando não interativo de
  inspeção que exiba o conteúdo atual do config. O formato padrão MUST
  ser humano legível (agrupado por seção, sem ruído). O comando MUST
  aceitar uma flag de formato (por exemplo `--format=<humano|json|toml>`)
  que altera a saída para JSON ou TOML determinísticos — sem cores,
  sem cabeçalhos decorativos — de modo a permitir consumo direto por
  scripts (`jq`, redirecionamento, comparação diff).
- **FR-011**: Se o config não existir, o comando de inspeção MUST
  exibir mensagem orientando executar a inicialização e MUST sair com
  código de erro.
- **FR-012**: Se o config existir mas contiver TOML inválido, o
  comando de inspeção MUST exibir a natureza do erro em linguagem
  humana (sem exposição de detalhes internos técnicos) e sugerir a
  reinicialização.

**Comando de alteração pontual**

- **FR-013**: O sistema MUST oferecer um comando não interativo que
  altere exatamente uma chave do config sem afetar as demais.
- **FR-014**: As chaves suportadas para alteração pontual MUST ser
  expressas em **notação dotted path** correspondendo 1:1 à estrutura
  do arquivo TOML: `paths.templates_dir`, `paths.output_dir`,
  `compiler.engine`, `compiler.keep_tex`, `compiler.keep_logs` e
  `behavior.ask_output_path_every_time`. Não são aceitos aliases,
  formas curtas ou variações.
- **FR-015**: Se a chave informada não for reconhecida (não estiver
  na lista canônica em notação dotted), o sistema MUST listar as
  chaves aceitas e sair com erro sem modificar o arquivo.
- **FR-016**: Para chaves de caminho, o sistema MUST expandir
  referências ao diretório home (`~`) e caminhos relativos para
  absolutos antes de gravar.
- **FR-017**: Para chaves booleanas, o sistema MUST aceitar apenas
  valores booleanos claros (rejeitando entrada ambígua) e MUST
  reportar erro amigável quando o valor for inválido.
- **FR-018**: Se o valor alterado for um caminho para um diretório que
  não existe, o sistema MUST persistir a mudança mas emitir aviso não
  bloqueante.
- **FR-019**: Se o valor alterado for um compilador ainda não
  suportado na versão atual, o sistema MUST persistir a mudança mas
  emitir aviso não bloqueante.
- **FR-020**: O comando de alteração MUST falhar com mensagem
  orientadora se nenhum config existir, sem criar um config parcial.

**Comportamento comum**

- **FR-021**: Todos os erros de usuário (config ausente, chave
  inválida, permissão negada, TOML corrompido) MUST ser exibidos em
  linguagem humana e ir para o canal de erro padrão, sem expor detalhes
  técnicos internos.
- **FR-022**: Todos os comandos desta feature MUST ter códigos de
  saída determinísticos: sucesso é código 0; erros de usuário e do
  ambiente têm códigos distintos e documentados.
- **FR-023**: Nenhum comando desta feature MUST modificar templates,
  PDFs ou arquivos fora do próprio arquivo de config (e, quando
  autorizado explicitamente, do diretório de templates informado).
- **FR-024**: Todos os comandos desta feature MUST ser **silenciosos
  por padrão** (apenas saída essencial em stdout, erros em stderr).
  Uma flag global `-v`/`--verbose`, repetível, MUST elevar
  progressivamente o nível de log: `-v` = informativo, `-vv` = debug,
  `-vvv` = trace. Mensagens de erro para o usuário MUST ir para stderr
  independentemente do nível de verbosidade — nunca são suprimidas.

### Key Entities

- **Config do Tex**: representa as preferências persistentes do
  usuário. Composto por três agrupamentos: caminhos (diretório de
  templates, diretório padrão de saída), preferências de compilador
  (engine escolhida, manter ou não o `.tex` intermediário, manter ou
  não logs de compilação) e comportamentos gerais (perguntar caminho
  de saída a cada conversão ou não). Persistido em um único arquivo TOML.
- **Chave de config**: identificador em notação de caminho
  (`compiler.engine`, `paths.templates_dir` etc.) usado para localizar
  um valor específico dentro do Config. O comando de alteração pontual
  opera sobre chaves.
- **Compilador LaTeX**: identificado por nome (`tectonic`, `latexmk`,
  `pdflatex`, `xelatex`, `lualatex`). Nesta versão da ferramenta, apenas
  `tectonic` é totalmente suportado; os demais são aceitos como
  preferência mas geram aviso.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Um usuário novo consegue completar a primeira
  configuração respondendo apenas aos prompts em menos de 30 segundos,
  contando desde a invocação do comando de inicialização até a
  mensagem final de confirmação.
- **SC-002**: O comando de inspeção retorna a saída completa em menos
  de 100 milissegundos (percebido pelo usuário como instantâneo) em
  hardware Linux comum.
- **SC-003**: Após uma alteração pontual, exatamente uma chave do
  arquivo de config muda; as demais permanecem byte-a-byte idênticas
  (excluindo diferenças cosméticas de formatação impostas pelo
  serializador).
- **SC-004**: Em cenário de config ausente, corrompido ou de chave
  inválida, o usuário consegue identificar sozinho o próximo passo com
  base apenas na mensagem exibida, sem consultar documentação externa
  em pelo menos 90% dos casos observados.
- **SC-005**: Nenhum comando desta feature causa perda silenciosa de
  configuração anterior — toda operação destrutiva é precedida de
  confirmação explícita ou é reversível pela reinicialização.
- **SC-006**: Um script CI simples (não interativo) consegue ler o
  config vigente e alterar uma chave sem intervenção humana, usando
  apenas os comandos de inspeção e alteração.

## Assumptions

- O usuário está em Linux com um diretório home padrão gravável e um
  caminho de configuração de usuário reconhecido pelo sistema
  operacional (conforme convenção XDG).
- O usuário utiliza a ferramenta em modo single-user e single-process;
  não há acesso concorrente ao arquivo de config.
- O compilador padrão da primeira versão é `tectonic`; a arquitetura
  reconhece outros nomes de engine como preferência do usuário, mas o
  suporte funcional a esses outros compiladores é escopo de features
  futuras.
- As entradas de caminho fornecidas pelo usuário são caminhos válidos
  do sistema de arquivos (não são URLs, caminhos remotos ou pontos de
  montagem exóticos).
- O arquivo de config é considerado propriedade única do Tex — edições
  manuais pelo usuário são permitidas, mas não é obrigação da
  ferramenta preservar comentários ou formatação arbitrária adicionada
  fora do fluxo dos comandos.
- Esta feature entrega apenas persistência e gestão do config; ela não
  usa o config para renderizar ou compilar nada — isso é escopo das
  duas features seguintes do MVP (Pipeline e Menu Interativo).
- Testes automatizados que exercitam interação com binários externos
  (por exemplo, verificar se `tectonic` está instalado) serão
  executados em um ambiente Linux isolado (container Debian) para
  garantir reprodutibilidade e não depender do estado do host; artefatos
  gerados nesse container são recuperados via cópia explícita. Isto é
  uma decisão de infraestrutura de teste e não afeta o comportamento
  observável da feature.
