Especificações de Modificações e Interface

*Nota: Tudo o que estiver entre parênteses é um comentário explicativo/orientação interna e não deve entrar no projeto final.*

# 1\. Barra de Menus

## 1.1 Arquivo

- **Últimos Arquivos**  
- **Novo**: *Ctrl \+ N*  
- **Abrir**: *Ctrl \+ O*  
- **Salvar**: *Ctrl \+ S* (Caso seja a primeira vez salvando, solicitará o formato. Por padrão, salva em JSON).  
- **Salvar Como**: *Ctrl \+ Shift \+ S* (Comportamento padrão com os formatos suportados: JSON, TOML, QET).  
- **Salvar Cópia**: *Ctrl \+ Alt \+ S*  
- **Fechar**: *Ctrl \+ W*  
- **Exportar**: *Ctrl \+ X* (Formatos: PNG, JPG, BMP, SVG, DXF, CAD, PDF. Opções: Toda página ou seleção; para o formato CAD, devido à limitação de não suportar múltiplas páginas, permite selecionar uma página específica para exportação).  
- **Imprimir**: *Ctrl \+ P*

## 1.2 Editar

- **Desfazer**: *Ctrl \+ Z*  
- **Refazer**: *Ctrl \+ Shift \+ Z*  
- **Cortar**: *Ctrl \+ X*  
- **Copiar**: *Ctrl \+ C*  
- **Colar**: *Ctrl \+ V* (Cola na posição do ponteiro do mouse e confirma o local com clique).  
- **Colar na Posição Original**: *Ctrl \+ Shift \+ V*  
- **Duplicar**: *Ctrl \+ D*  
- **Inserir o Último Elemento**: *A*  
- **Colorir Elemento** (Abre janela de seleção de cor para o elemento; substitui a cor preta em tema light ou branca em tema dark pela cor escolhida).  
- **Adicionar**  
  - Adicionar Texto em Linha  
  - Adicionar Campo de Texto  
  - Adicionar Imagem  
  - Adicionar Linha  
  - Adicionar Retângulo  
  - Adicionar Círculo  
  - Adicionar Arco  
  - *(Outros itens serão adicionados no futuro)*  
- **Seleção**  
  - Selecionar Tudo	Ctrl \+ A  
  - Anular a Seleção	Ctrl \+ Shift \+ A  
  - Inverter Seleção	Ctrl \+ I  
- **Copiar Informação do Elemento**: *Ctrl \+ Alt \+ C*  
- **Colar Informação do Elemento**: *Ctrl \+ Alt \+ V*  
- **Deletar**: *Del*  
- **Rotacionar**: *R* (Incrementos de 90°: 0°, 90°, 180°, 270°).  
- **Rotacionar como Grupo**: *Shift \+ R*  
- **Espelhar Horizontalmente**: *M*  
- **Espelhar Verticalmente**: *F*  
- **Agrupar**  
  - Agrupar	Ctrl \+ G  
  - Desagrupar	Ctrl \+ U  
- **Linhas e Colunas**  
  - Adicionar uma Linha  
  - Apagar uma Linha  
  - Adicionar uma Coluna  
  - Apagar uma Coluna  
- **Propriedades da Página**: *Ctrl \+ Shift \+ Enter*

## 1.3 Projeto

- **Propriedades do Projeto** *(A definir)*  
- **Criação Automática de Condutores** (Traça e conecta automaticamente condutores aos elementos quando os bornes estão alinhados vertical ou horizontalmente. **Ligado por padrão**).  
- **Desconexão Automática de Condutores** (Mantém o condutor estático ao mover o elemento, desligando-o dos bornes. **Desligado por padrão**).  
- **Adicionar uma Página**: *Ctrl \+ T*  
- **Deletar esta Página**: *Ctrl \+ Shift \+ BackSpace*  
- *(Definições de exportação do projeto como CSV, numeração de conectores, BOM, etc., serão tratadas futuramente).*

## 1.4 Display (Exibição)

- **Mostrar Projetos**  
- **Selecionar** (Habilita/Desabilita a Seleção de Elementos. Ligado por padrão; mutuamente exclusivo com "Mover").  
- **Mover** (Move o Canvas de Trabalho livremente mantendo o botão direito do mouse pressionado).  
- **Mostrar a Grelha**  
- **Ajustar à Grelha**: *Ctrl \+ Shift \+ G*  
- **Passo da Grelha**  
- **Mira em Todo o Fólio**  
- **Grelha de Texto**  
- **Ver os Guias**  
- **Manter Cabeçalhos Visíveis**  
- **Mostrar Limites das Células**  
- **Mostrar**  
- **Cor de Fundo da Folha**  
- **Ampliar (Zoom In)**: *Ctrl \+ \+*  
- **Reduzir (Zoom Out)**: *Ctrl \+ \-*  
- **Zoom do Conteúdo**: *Ctrl \+ 8*  
- **Ajustar à Vista**: *Ctrl \+ 9*  
- **Repor Zoom**: *Ctrl \+ 0*  
- **Centrar no Cursor**

## 1.5 Outros Menus

*Ainda a incluir: Configurações (Settings), Janela (Window) e Ajuda (Help).*

*A Barra de Ferramentas (Toolbar) contém apenas ícones; os rótulos de texto são exibidos via hover (passagem do mouse).*

# 2\. Barra de Ferramentas (Toolbar)

- Novo  
- Abrir  
- Salvar  
- Salvar Como  
- Fechar  
- Imprimir  
- Exportar em PDF  
- Desfazer  
- Refazer  
- Cortar  
- Copiar  
- Colar  
- Duplicar  
- Deletar  
- Rotacionar  
- Espelhar Horizontalmente  
- Espelhar Verticalmente  
- Colorir Elemento  
- Selecionar  
- Mover  
- **Mostrar a Grelha** (Ligado por padrão)  
- **Snap à Grelha** (Ligado por padrão)  
- **Seleção de Densidade da Grelha** (Menu Dropdown):  
  - Fina (10 pts por cm)  
  - Média (5 pts por cm)  
  - Larga (2.5 pts por cm)  
- **Textos** (Menu Dropdown):  
  - Ligado  
  - Desligado  
- **Tema**: Modo Claro, Escuro e Sistema

# 3\. Elementos e Projetos

A barra lateral esquerda, anteriormente denominada "Coleções/Projetos", passa a se chamar **Elementos e Projetos**.  
Na parte inferior da barra, devem estar presentes dois seletores de aba: **Elementos** e **Projetos**.

## 3.1 Aba "Elementos"

Esta aba fica selecionada por padrão. Exibe a estrutura de símbolos utilitários atual. O comportamento de inserção no Canvas de Trabalho é o seguinte:

- **Clique duplo no nome do símbolo, ou clique em "Inserir no Esquema"**: Habilita o símbolo no ponteiro do mouse; no próximo clique, ele é posicionado no local desejado.  
- **Arrastar o nome do símbolo**: O símbolo é colocado diretamente no Canvas de Trabalho na posição de soltura.  
- **Cancelamento**: Um clique com o botão direito ou a tecla *ESC* cancela a ação e retorna o ponteiro ao modo de seleção.

## 3.2 Aba "Projetos"

Exibe os últimos projetos trabalhados pelo usuário:

- **Abertura**: Clique duplo sobre o projeto ou clique no botão "Abrir".  
- **Exclusão**: Pressionar a tecla *Delete* ou clicar no botão "Apagar". Exibe janela de confirmação antes de apagar.  
- **Navegação de Páginas**: Exibe menu dropdown com as páginas no formato: *Nome do Projeto \-\> (Nº da Página / Total de Páginas) Nome da Página*.

## 3.3 Recolhimento da Barra

A barra lateral deve possuir um ícone chevron para colapso. Quando recolhida, o ícone permanece visível para possibilitar a restauração da barra.

# 4\. Propriedades do Elemento

A barra lateral direita mantém o nome **Propriedades do Elemento** e disponibiliza dois seletores principais: **Propriedades** (anteriormente "Informações") e **Referência Cruzada**.

## 4.1 Seletor: Propriedades

Quando um elemento está selecionado, exibe os campos editáveis abaixo:

- Nome:  
- Função:  
- **Borne N**: Identificação de cada borne do elemento, seguindo as normas **IEC 60947-1** e **IEC 81346** para designação estruturada:  
  -   
  - 1\. Circuitos de Potência (Força)  
  -   
  - **Circuitos de Potência (Força)** (Contatores, disjuntores-motores, relés térmicos – 1 dígito):  
    - **Números Ímpares (1, 3, 5 ou L1, L2, L3)**: Bornes de entrada (alimentação das fases).  
  - **Números Pares (2, 4, 6 ou T1, T2, T3)**: Bornes de saída (carga/motor).  
  -   
  - **Circuitos de Comando** (Contatos auxiliares – 2 dígitos):  
  -   
  - Nos contactos auxiliares de comando, os bornes utilizam dois dígitos:  
    - **1º Dígito**: Número de sequência do contato no aparelho (1º, 2º, etc.).  
    - **2º Dígito**: Função elétrica do contato:  
      - **1 e 2**: Normal Fechado (NF / NC) (Ex: 11-12, 21-22).  
      - **3 e 4**: Normal Aberto (NA / NO) (Ex: 13-14, 23-24).  
      - **5 e 6**: NF especial (temporizadores, sobrecarga).  
  - **7 e 8**: NA especial (Ex: 97-98 no relé térmico para sinalização de falha).  
  -   
  - **Bobinas e Sinalizadores**:  
  -   
    - **A1 e A2**: Bobinas de comando (Contatores/Relés).  
    - **X1 e X2**: Sinalizadores luminosos / Lâmpadas.  
  - **Identificação de Réguas de Bornes em Painéis (IEC 81346\)**: Designadas pela letra **X**:  
  - Tabela de subdivisão de funções de réguas de bornes:

| Identificação da Régua | Função Principal | Tipo de Circuito |
| :---- | :---- | :---- |
| XP (ou X0) | Bornes destinados à alimentação geral e motores | Potência / Força |
| XC | Bornes para interligação de botoeiras, sensores e lógicas | Comando / Auxiliar |
| XT | Bornes dedicados a transformadores e medição | Auxiliar |

- Bobinas com padrões especiais devem apresentar esses padrões para edição.  
- Sinalizadores devem apresentar a cor para escolher  
- Sinalizadores Pulsantes devem escolher o tempo de pulso

## 4.2 Seletor: Referência Cruzada

- **Nenhum elemento selecionado**: Exibe a referência cruzada baseada em nomes de todo o projeto.  
- **Elemento selecionado**: Exibe apenas as referências cruzadas do elemento selecionado, considerando nome e ligação de bornes (em série e em paralelo).

# 5\. Janela Principal (Canvas de Trabalho)

A área central do Canvas de Trabalho contém a barra de título dividida em duas linhas estruturadas:

## 5.1 Primeira Linha: Barra de Título

- **Centro**: Nome do Esquema aberto. Caso o projeto tenha sido modificado e não salvo, recebe a indicação **\[Modificado\]**.  
- **Canto Direito**: Três ícones em formato quadrado (Novo, Abrir e Fechar), com altura equivalente à da linha.

## 5.2 Segunda Linha: Barra de Abas de Páginas

- **Estrutura das Abas**: Exibe o número da página seguido pelo nome da página.  
- **Edição do Nome**: Clique duplo seleciona o nome da página para edição direta.  
- **Menu de Contexto (Botão Direito)**: Apresenta as opções:  
  - Editar nome da página;  
  - Editar número da página (quando houver mais de uma);  
  - Deletar página (exibe aviso de fechamento do projeto e oferece salvamento se restar apenas uma página);  
  - Criar página antes;  
  - Criar página depois.

**Atalhos das Extremidades**: Ícone no canto esquerdo para criar página antes; ícone no canto direito para criar página depois.

## 5.3 Padrão de Exibição de Temas do Canvas

O Canvas de Trabalho deve respeitar a seleção de temas (LIGHT, DARK e SYSTEM):

- **Tema LIGHT**: Exibe a folha de trabalho clara com os elementos e traços escuros, os conectores, caso o utilizador não os tenha mudado de cor também devem seguir este padrão.  
- **Tema DARK**: Exibe a folha de trabalho escura com os elementos e traços clarosos conectores, caso o utilizador não os tenha mudado de cor também devem seguir este padrão.

**Tema SYSTEM**: Alterna automaticamente entre os modos LIGHT e DARK acompanhando a preferência do sistema operacional.