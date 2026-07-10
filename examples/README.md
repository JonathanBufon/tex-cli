# examples/templates/

Coleção **curada** de templates LaTeX standalone (um único `.tex`
autocontido, sem `.cls` externo nem subdirs) para popular
rapidamente o `paths.templates_dir` do `tex-cli`.

## Origem e atribuição

Os templates foram escritos ou adaptados **inspirados no trabalho de
[Eranot](https://github.com/Eranot)**, autor do
[Unotex](https://github.com/Eranot/Unotex) — modelo LaTeX para
trabalhos acadêmicos da Universidade Comunitária da Região de
Chapecó (Unochapecó), publicado sob a **LaTeX Project Public
License, Version 1.3c (LPPL 1.3c)**.

Créditos:

- **Autor original**: Eranot (`https://github.com/Eranot`)
- **Projeto de origem**: Unotex (`https://github.com/Eranot/Unotex`)
- **Licença**: LPPL 1.3c (texto integral em
  [`../LICENSE-EXAMPLES`](../LICENSE-EXAMPLES))

Os templates deste diretório são **derivações standalone
simplificadas** do Unotex — eles **não** dependem do arquivo
`unotex.cls` nem dos subdiretórios do projeto original
(`introducao/`, `pre-textuais/`, `bibliografia/`, `img/`), e são
distribuídos aqui pra servir de ponto de partida para quem quer
usar o `tex-cli` sem antes montar um projeto LaTeX completo. Para o
projeto completo com classe própria, consulte o repositório
original.

Se você reusar/publicar estes templates fora deste repositório,
mantenha a atribuição ao Eranot e o texto da LPPL 1.3c.

## Como usar

### Copiar todos direto para o `templates_dir` do config

```bash
# Descobre o templates_dir configurado:
DEST=$(tex-cli config show --format json | jq -r .paths.templates_dir)

# Copia:
cp examples/templates/*.tex "$DEST/"

# Confirma:
tex-cli templates list
```

### Adicionar via `tex-cli templates add`

```bash
tex-cli templates add examples/templates/artigo-basico.tex
tex-cli templates add examples/templates/carta.tex
```

Cada `add` valida UTF-8, grava atomicamente com permissão `0644` e
imprime o caminho final.

## Templates incluídos

| Arquivo               | Descrição                                                   |
|-----------------------|-------------------------------------------------------------|
| `artigo-basico.tex`   | Artigo científico curto (título/autor/resumo/introdução).   |
| `carta.tex`           | Carta formal em ABNT (destinatário/assunto/corpo).          |

Adicione novos exemplos abrindo PR com o `.tex` standalone + linha
correspondente nesta tabela. Se o novo template derivar de trabalho
sob licença específica (LPPL, CC-BY, MIT-tex, etc.), documente a
atribuição neste arquivo antes de commitar.

## Dados de exemplo (`data/`)

Cada template acima tem um JSON correspondente em `examples/data/`
com valores realistas prontos pra alimentar o `tex-cli render`:

| Data JSON                          | Alimenta o template   |
|------------------------------------|-----------------------|
| `data/artigo-basico.json`          | `templates/artigo-basico.tex` |
| `data/carta.json`                  | `templates/carta.tex` |

Receita rápida (assume config gerado + template adicionado):

```bash
tex-cli templates add examples/templates/artigo-basico.tex
tex-cli render artigo-basico examples/data/artigo-basico.json
```

Ou tudo num dry-run pra ver o `.tex` renderizado antes de gravar:

```bash
tex-cli render artigo-basico examples/data/artigo-basico.json --dry-run | less
```

## Licença

`../LICENSE-EXAMPLES` contém o texto integral da LPPL 1.3c
(`https://www.latex-project.org/lppl/lppl-1-3c.txt`).

O código Rust em `../src/` e os documentos de spec em `../specs/`
seguem a licença do repositório principal (ver `../LICENSE`).
