<!--
Sync Impact Report
Version change: (template) → 1.0.0
Bump rationale: Initial ratification of Tex constitution (MAJOR by convention for first release).

Modified principles:
  - (all new — first ratification)

Added sections:
  - Core Principles (I. Escopo Pessoal e Não-Substituição do LaTeX;
                    II. Separação Estrita de Dados, Template e PDF;
                    III. Interface Dupla — CLI Direto e Menu Interativo;
                    IV. Compilador LaTeX Plugável com Tectonic Padrão;
                    V. Segurança na Manipulação de Arquivos)
  - Restrições Técnicas (stack fixada, plataforma alvo, config path)
  - Governança (licença, versionamento, emendas)

Removed sections: none

Templates alignment:
  ✅ .specify/templates/plan-template.md — Constitution Check é genérico, sem mudança necessária;
     as gates serão preenchidas em cada /speckit-plan a partir dos princípios abaixo.
  ✅ .specify/templates/spec-template.md — sem hardcode a princípios; nenhuma seção mandatória
     conflita com esta constitution.
  ✅ .specify/templates/tasks-template.md — organização por user story permanece compatível;
     tarefas de segurança/log virão dos princípios V ao serem geradas.
  ✅ .specify/templates/checklist-template.md — não inspecionado como bloqueador; genérico.
  ✅ CLAUDE.md do projeto — mantém referência genérica ao "current plan"; nenhum ajuste requerido.

Deferred items: nenhum.
-->

# Constituição do Tex

O **Tex** é uma ferramenta de terminal, escrita em Rust, que converte dados
estruturados em JSON para documentos LaTeX e compila esses documentos para PDF.
Esta constituição fixa os princípios não-negociáveis do projeto e serve como
gate para todas as specs, plans, tarefas e revisões futuras.

## Core Principles

### I. Escopo Pessoal e Não-Substituição do LaTeX

O Tex é uma ferramenta de **uso pessoal** voltada à geração de artigos
científicos, relatórios técnicos e documentos padronizados a partir de
templates reaproveitáveis. O projeto **NÃO** busca substituir o LaTeX nem
oferecer alternativa ao ecossistema `tectonic` / `latexmk` / `pdflatex`; atua
como **camada de automação** sobre esse ecossistema.

Regras derivadas:

- Features MUST responder à pergunta "isto ajuda um usuário individual a gerar
  um documento LaTeX mais rápido?" — caso contrário, ficam fora do escopo.
- Integrações com sistemas corporativos (CASIA, LMS, CI proprietário) MUST ser
  rejeitadas na v1; podem ser reavaliadas apenas após ratificação de nova versão
  da constituição.
- Reimplementação de mecanismos internos do LaTeX (parsing de `.tex`,
  tipografia, resolução de packages) é **proibida**. O Tex delega essas
  responsabilidades ao compilador configurado.

**Rationale**: Manter o escopo estreito impede o projeto de degenerar em um
concorrente do LaTeX; a proposta de valor é a automação da ponte JSON → PDF, e
qualquer desvio dilui essa proposta.

### II. Separação Estrita de Dados, Template e PDF

Três artefatos são reconhecidos e nunca misturados:

```text
JSON (dados) → Template .tex (apresentação) → PDF (saída)
```

Regras derivadas:

- Templates `.tex` MUST ser tratados como arquivos-fonte estáticos do usuário e
  NUNCA gerados ou modificados pelo Tex em tempo de execução.
- Dados MUST permanecer em JSON puro; o Tex NÃO aceita outros formatos de
  entrada (YAML, TOML, front-matter em Markdown) na v1.
- O PDF é sempre um artefato **derivado**; nunca é entrada para nenhum comando.
- O `.tex` intermediário gerado pelo pipeline é **derivado** dos dois artefatos
  primários e pode ser mantido ou descartado conforme configuração — mas nunca
  editado manualmente como parte do fluxo suportado.

**Rationale**: A separação clara resolve o problema central identificado na
descrição do projeto (repetição de templates, edição manual arriscada, difícil
reaproveitamento). Qualquer feature que borre essa separação MUST ser rejeitada.

### III. Interface Dupla — CLI Direto e Menu Interativo

O Tex expõe **duas interfaces de igual prioridade**:

1. **CLI direto** — subcomandos `init`, `config`, `templates`, `render`,
   `compile`, `build` — otimizado para automação, scripts e uso repetido.
2. **Menu interativo** — comando `tex-cli interactive` (ou `tex-cli` sem
   argumentos) — otimizado para uso manual e primeira experiência.

Regras derivadas:

- Toda funcionalidade acessível pelo menu interativo MUST também ser acessível
  por um comando CLI direto (a inversa não é obrigatória; comandos de baixo
  nível como `render` podem ficar fora do menu).
- O modo CLI direto MUST ser **scriptável**: exit codes bem definidos, saída
  determinística, sem prompts interativos quando todos os argumentos são
  fornecidos.
- Prompts interativos MUST usar `inquire`; parsing de argumentos MUST usar
  `clap`. Não misturar responsabilidades.
- Erros em modo CLI MUST ir para `stderr`; sucesso vai para `stdout`.

**Rationale**: A dualidade é parte da proposta — automação para o dia-a-dia,
menu para configuração ocasional ou usuários novos. Priorizar uma sobre a
outra quebra o contrato do produto.

### IV. Compilador LaTeX Plugável com Tectonic Padrão

O compilador LaTeX é um **componente substituível** por design.

Regras derivadas:

- A v1 MUST suportar `tectonic` como engine padrão (mais simples, sem
  instalação LaTeX completa, boa DX para CLI).
- O código MUST isolar a invocação do compilador em um módulo dedicado
  (`compiler.rs`) com trait/enum de engine, de forma que adicionar `latexmk`,
  `pdflatex`, `xelatex` ou `lualatex` NÃO exija refactor arquitetural.
- A escolha do engine MUST ser expressa no arquivo de configuração
  (`[compiler].engine`) — nunca hardcoded fora do módulo `compiler.rs`.
- A presença do binário do compilador MUST ser verificada via `which` antes da
  execução, com mensagem de erro clara caso ausente.

**Rationale**: Escolher tectonic como padrão dá simplicidade sem prender o
usuário. A extensibilidade é uma promessa arquitetural, não um "vamos ver
depois" — tem que estar no design desde a v1.

### V. Segurança na Manipulação de Arquivos

O Tex lida com arquivos do usuário (dados de entrada, templates, PDFs de
saída). Comportamento seguro é obrigatório e não uma feature opcional.

Regras não-negociáveis:

- O Tex MUST NUNCA sobrescrever um PDF existente sem confirmação explícita
  (interativa via `inquire`, ou flag como `--force` no modo direto).
- O JSON de entrada MUST ser validado (parse bem-sucedido) **antes** de
  qualquer chamada ao renderer.
- A existência do template selecionado MUST ser verificada antes do render.
- O diretório de saída MUST ser verificado; se ausente, o Tex MAY criá-lo
  apenas com consentimento (interativo) ou flag equivalente.
- Erros de compilação MUST ser apresentados em formato humano legível — nunca
  vazar stack trace bruto do Rust ao usuário final.
- Logs de compilação MUST ser preservados quando o compilador os produzir e
  quando `[compiler].keep_logs = true`.
- O `.tex` intermediário MAY ser preservado ou descartado conforme
  `[compiler].keep_tex`.

**Rationale**: Perder um PDF por sobrescrita silenciosa, ou receber um erro
opaco após uma compilação demorada, são falhas de UX que destroem a confiança
na ferramenta. Segurança nesses pontos é gate obrigatório para qualquer PR.

## Restrições Técnicas

Estas restrições são **fixas para a v1** e alterá-las requer emenda formal
(ver Governança).

### Linguagem e distribuição

- Linguagem única: **Rust** (edição estável mais recente à data da spec).
- Artefato: **binário executável** distribuído via `cargo install` e via
  releases do GitHub.
- Plataforma alvo primária: **Linux**. macOS e Windows são desejáveis, mas
  NÃO devem influenciar decisões de arquitetura da v1; nenhum código
  específico de Linux deve ser adicionado sem justificativa.

### Stack de bibliotecas fixada

As dependências abaixo são **canônicas**. Substituí-las requer justificativa
na spec ou emenda à constituição:

| Biblioteca            | Uso                                                |
| --------------------- | -------------------------------------------------- |
| `clap`                | parsing de comandos e argumentos                   |
| `inquire`             | prompts e menu interativo                          |
| `serde` + `serde_json`| desserialização e validação de JSON                |
| `tera`                | aplicação de dados no template `.tex`              |
| `toml`                | leitura e escrita do config                        |
| `dirs`                | resolução multiplataforma de `~/.config/tex`       |
| `tempfile`            | arquivos temporários de compilação                 |
| `anyhow`              | erros em fronteiras da aplicação (main, comandos)  |
| `thiserror`           | tipos de erro internos dos módulos                 |
| `tracing`             | logs estruturados                                  |
| `which`               | verificação de presença de binários (tectonic etc.)|

### Configuração e paths

- Path canônico do config: `~/.config/tex/config.toml`
  (resolvido via `dirs::config_dir()`).
- Nenhum path MUST ser hardcoded no código — todos passam por um resolver
  centralizado (`paths.rs`).
- Diretórios padrão de templates e output MUST vir do config; se ausente, o
  Tex MUST orientar o usuário a rodar `tex-cli init` ao invés de silenciar.

### Estrutura interna esperada

A árvore alvo é a que consta na descrição do projeto (`src/main.rs`,
`cli.rs`, `config.rs`, `interactive.rs`, `templates.rs`, `renderer.rs`,
`compiler.rs`, `errors.rs`, `paths.rs`). Divergências MUST ser justificadas
na spec/plan da feature correspondente.

## Governança

### Autoridade da constituição

Esta constituição **prevalece** sobre qualquer prática ad hoc, decisão
individual em PR ou preferência estética. Todas as specs (`spec.md`), plans
(`plan.md`) e revisões MUST ser checadas contra esta constituição no Constitution
Check antes de serem aprovadas.

### Licenciamento

O Tex é **software livre e de código aberto**, distribuído sob a licença já
presente no repositório (arquivo `LICENSE`). Contribuições MUST ser
compatíveis com essa licença.

### Versionamento da constituição

A constituição segue **SemVer** próprio:

- **MAJOR**: remoção ou redefinição incompatível de princípio existente.
- **MINOR**: adição de novo princípio ou expansão material de guidance.
- **PATCH**: clarificações, correções de wording, ajustes não-semânticos.

### Processo de emenda

1. Emendas MUST ser propostas em uma spec ou PR dedicado.
2. O Sync Impact Report MUST ser atualizado no cabeçalho deste arquivo.
3. Templates dependentes (`plan-template.md`, `spec-template.md`,
   `tasks-template.md`, `checklist-template.md`) MUST ser revisados na mesma
   mudança e marcados como ✅ ou ⚠ no relatório.
4. A data de `Last Amended` MUST ser atualizada.

### Constitution Check em specs e plans

Ao rodar `/speckit-plan`, o Constitution Check MUST validar minimamente:

- A feature respeita o Princípio I (escopo pessoal, não substitui LaTeX)?
- Mantém a separação JSON / .tex / PDF do Princípio II?
- Se expõe funcionalidade, oferece CLI direto **e** (quando aplicável) menu
  interativo, conforme Princípio III?
- Se toca o compilador, mantém o isolamento do Princípio IV?
- Cumpre todas as regras de segurança do Princípio V?
- Usa apenas bibliotecas da stack fixada, ou justifica qualquer adição?

Violações identificadas MUST ser listadas na tabela de Complexity Tracking do
plan com justificativa e alternativa simples rejeitada.

### Guia de runtime

Para orientações de execução dia-a-dia (comandos, convenções, decisões
concretas por feature), consultar o `plan.md` da feature em curso, referenciado
pelo `CLAUDE.md` do projeto.

**Version**: 1.0.0 | **Ratified**: 2026-07-09 | **Last Amended**: 2026-07-09
