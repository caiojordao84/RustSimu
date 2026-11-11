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

- **Desfazer**: *Ctrl \+ Z* [CONCLUÍDO]  
- **Refazer**: *Ctrl \+ Shift \+ Z* [CONCLUÍDO]  
- **Cortar**: *Ctrl \+ X* [CONCLUÍDO]  
- **Copiar**: *Ctrl \+ C* [CONCLUÍDO]  
- **Colar**: *Ctrl \+ V* (Cola na posição do ponteiro do mouse e confirma o local com clique). [CONCLUÍDO]  
- **Colar na Posição Original**: *Ctrl \+ Shift \+ V* [CONCLUÍDO]  
- **Duplicar**: *Ctrl \+ D* [CONCLUÍDO]  
- **Inserir o Último Elemento**: *A* [CONCLUÍDO]  
- **Colorir Elemento** (Abre janela de seleção de cor para o elemento; substitui a cor preta em tema light ou branca em tema dark pela cor escolhida). [CONCLUÍDO]  
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
  - Selecionar Tudo	Ctrl \+ A [CONCLUÍDO]  
  - Anular a Seleção	Ctrl \+ Shift \+ A [CONCLUÍDO]  
  - Inverter Seleção	Ctrl \+ I [CONCLUÍDO]  
- **Copiar Informação do Elemento**: *Ctrl \+ Alt \+ C* [CONCLUÍDO]  
- **Colar Informação do Elemento**: *Ctrl \+ Alt \+ V* [CONCLUÍDO]  
- **Deletar**: *Del* [CONCLUÍDO]  
- **Rotacionar**: *R* (Incrementos de 90°: 0°, 90°, 180°, 270°). [CONCLUÍDO]  
- **Rotacionar como Grupo**: *Shift \+ R* [CONCLUÍDO]  
- **Espelhar Horizontalmente**: *M* [CONCLUÍDO]  
- **Espelhar Verticalmente**: *F* [CONCLUÍDO]  
- **Agrupar**  
  - Agrupar	Ctrl \+ G [CONCLUÍDO]  
  - Desagrupar	Ctrl \+ U [CONCLUÍDO]  
- **Linhas e Colunas**  
  - Adicionar uma Linha  
  - Apagar uma Linha  
  - Adicionar uma Coluna  
  - Apagar uma Coluna  
- **Propriedades da Página**: *Ctrl \+ Shift \+ Enter* [CONCLUÍDO]

## 1.3 Projeto

- **Propriedades do Projeto** *(A definir)*  
- **Criação Automática de Condutores** (Traça e conecta automaticamente condutores aos elementos quando os bornes estão alinhados vertical ou horizontalmente. **Ligado por padrão**). [CONCLUÍDO]  
- **Desconexão Automática de Condutores** (Mantém o condutor estático ao mover o elemento, desligando-o dos bornes. **Desligado por padrão**). [CONCLUÍDO]  
- **Adicionar uma Página**: *Ctrl \+ T* [CONCLUÍDO]  
- **Deletar esta Página**: *Ctrl \+ Shift \+ BackSpace* [CONCLUÍDO]  
- *(Definições de exportação do projeto como CSV, numeração de conectores, BOM, etc., serão tratadas futuramente).*

## 1.4 Display (Exibição)

- **Mostrar Projetos**  
- **Selecionar** (Habilita/Desabilita a Seleção de Elementos. Ligado por padrão; mutuamente exclusivo com "Mover"). [CONCLUÍDO]  
- **Mover** (Move o Canvas de Trabalho livremente mantendo o botão direito do mouse pressionado). [CONCLUÍDO]  
- **Mostrar a Grelha** [CONCLUÍDO]  
- **Ajustar à Grelha**: *Ctrl \+ Shift \+ G* [CONCLUÍDO]  
- **Passo da Grelha**  
- **Mira em Todo o Fólio**  
- **Grelha de Texto**  
- **Ver os Guias**  
- **Manter Cabeçalhos Visíveis**  
- **Mostrar Limites das Células**  
- **Mostrar**  
- **Cor de Fundo da Folha**  
- **Ampliar (Zoom In)**: *Ctrl \+ \+* [CONCLUÍDO]  
- **Reduzir (Zoom Out)**: *Ctrl \+ \-* [CONCLUÍDO]  
- **Zoom do Conteúdo**: *Ctrl \+ 8* [CONCLUÍDO]  
- **Ajustar à Vista**: *Ctrl \+ 9* [CONCLUÍDO]  
- **Repor Zoom**: *Ctrl \+ 0* [CONCLUÍDO]  
- **Centrar no Cursor**

## 1.5 Outros Menus

*Ainda a incluir: Configurações (Settings), Janela (Window) e Ajuda (Help).*

*A Barra de Ferramentas (Toolbar) contém apenas ícones; os rótulos de texto são exibidos via hover (passagem do mouse).*

# 2\. Barra de Ferramentas (Toolbar) — [100% CONCLUÍDO - Opção A]

- Novo [CONCLUÍDO]  
- Abrir [CONCLUÍDO]  
- Salvar [CONCLUÍDO]  
- Salvar Como [CONCLUÍDO]  
- Fechar [CONCLUÍDO]  
- Imprimir [CONCLUÍDO]  
- Exportar em PDF [CONCLUÍDO]  
- Desfazer [CONCLUÍDO]  
- Refazer [CONCLUÍDO]  
- Cortar [CONCLUÍDO]  
- Copiar [CONCLUÍDO]  
- Colar [CONCLUÍDO]  
- Duplicar [CONCLUÍDO]  
- Deletar [CONCLUÍDO]  
- Rotacionar [CONCLUÍDO]  
- Espelhar Horizontalmente [CONCLUÍDO]  
- Espelhar Verticalmente [CONCLUÍDO]  
- Colorir Elemento [CONCLUÍDO - Janela modal com paleta industrial de 12 cores, entrada hex e visualização]  
- Selecionar [CONCLUÍDO - Ferramenta de seleção com exclusividade mútua frente ao pan]  
- Mover [CONCLUÍDO - Ferramenta de pan no canvas com cursor grab/grabbing e arrasto com botão direito]  
- **Mostrar a Grelha** (Ligado por padrão) [CONCLUÍDO]  
- **Snap à Grelha** (Ligado por padrão) [CONCLUÍDO]  
- **Seleção de Densidade da Grelha** (Menu Dropdown): [CONCLUÍDO]  
  - Fina (10 pts por cm)  
  - Média (5 pts por cm)  
  - Larga (2.5 pts por cm)  
- **Textos** (Menu Dropdown): [CONCLUÍDO]  
  - Ligado  
  - Desligado  
- **Tema**: Modo Claro, Escuro e Sistema [CONCLUÍDO]

# 3\. Elementos e Projetos — [100% CONCLUÍDO - Opção B]

A barra lateral esquerda, anteriormente denominada "Coleções/Projetos", passa a se chamar **Elementos e Projetos**. [CONCLUÍDO]  
Na parte inferior da barra, devem estar presentes dois seletores de aba: **Elementos** e **Projetos**. [CONCLUÍDO]

## 3.1 Aba "Elementos" — [100% CONCLUÍDO]

Esta aba fica selecionada por padrão. Exibe a estrutura de símbolos utilitários atual. O comportamento de inserção no Canvas de Trabalho é o seguinte:

- **Clique duplo no nome do símbolo, ou clique em "Inserir no Esquema"**: Habilita o símbolo no ponteiro do mouse; no próximo clique, ele é posicionado no local desejado. [CONCLUÍDO]  
- **Arrastar o nome do símbolo**: O símbolo é colocado diretamente no Canvas de Trabalho na posição de soltura. [CONCLUÍDO]  
- **Cancelamento**: Um clique com o botão direito ou a tecla *ESC* cancela a ação e retorna o ponteiro ao modo de seleção. [CONCLUÍDO]

## 3.2 Aba "Projetos" — [100% CONCLUÍDO]

Exibe os últimos projetos trabalhados pelo usuário:

- **Abertura**: Clique duplo sobre o projeto ou clique no botão "Abrir". [CONCLUÍDO]  
- **Exclusão**: Pressionar a tecla *Delete* ou clicar no botão "Apagar". Exibe janela de confirmação antes de apagar. [CONCLUÍDO]  
- **Navegação de Páginas**: Exibe menu dropdown com as páginas no formato: *Nome do Projeto \-\> (Nº da Página / Total de Páginas) Nome da Página*. [CONCLUÍDO]

## 3.3 Recolhimento da Barra — [100% CONCLUÍDO]

A barra lateral deve possuir um ícone chevron para colapso. Quando recolhida, o ícone permanece visível para possibilitar a restauração da barra. [CONCLUÍDO]

# 4\. Propriedades do Elemento — [100% CONCLUÍDO]

A barra lateral direita mantém o nome **Propriedades do Elemento** e disponibiliza dois seletores principais: **Propriedades** (anteriormente "Informações") e **Referência Cruzada**. [CONCLUÍDO]

## 4.1 Seletor: Propriedades — [100% CONCLUÍDO]

Quando um elemento está selecionado, exibe os campos editáveis abaixo:

- **Nome**: Campo editável para `item.tag` com checkbox "Exibir no esquema" (`item.show_tag`) e validação inteligente de vínculo pai/contatos e anti-duplicidade. [CONCLUÍDO]  
- **Função**: Campo editável para `item.function` (com fallback para `item.value`) com checkbox "Exibir no esquema" (`item.show_value`). [CONCLUÍDO]  
- **Borne N**: Identificação de cada borne do elemento, seguindo as normas **IEC 60947-1** e **IEC 81346** para designação estruturada: [CONCLUÍDO]  
  - **Circuitos de Potência (Força)** (Contatores, disjuntores-motores, relés térmicos – 1 dígito): [CONCLUÍDO]  
    - **Números Ímpares (1, 3, 5 ou L1, L2, L3)**: Bornes de entrada (alimentação das fases). [CONCLUÍDO]  
    - **Números Pares (2, 4, 6 ou T1, T2, T3)**: Bornes de saída (carga/motor). [CONCLUÍDO]  
    - Presets rápidos para 3P e 1P numérico e fases industriais (`1-2, 3-4, 5-6`, `L1-T1, L2-T2, L3-T3`, `1-2`, `L1-T1`). [CONCLUÍDO]  
  - **Circuitos de Comando** (Contatos auxiliares – 2 dígitos): [CONCLUÍDO]  
    - Nos contactos auxiliares de comando, os bornes utilizam dois dígitos: [CONCLUÍDO]  
      - **1º Dígito**: Número de sequência do contato no aparelho (1º, 2º, etc., 1..9). [CONCLUÍDO]  
      - **2º Dígito**: Função elétrica do contato: [CONCLUÍDO]  
        - **1 e 2**: Normal Fechado (NF / NC) (Ex: 11-12, 21-22). [CONCLUÍDO]  
        - **3 e 4**: Normal Aberto (NA / NO) (Ex: 13-14, 23-24). [CONCLUÍDO]  
        - **5 e 6**: NF especial (temporizadores, sobrecarga). [CONCLUÍDO]  
        - **7 e 8**: NA especial (Ex: 97-98 no relé térmico para sinalização de falha). [CONCLUÍDO]  
      - Dropdown sequencial inteligente IEC 60947 (1 a 9) com sugestão de dígito livre e detecção de colisões. [CONCLUÍDO]  
  - **Bobinas e Sinalizadores**: [CONCLUÍDO]  
    - **A1 e A2**: Bobinas de comando (Contatores/Relés). [CONCLUÍDO]  
    - **X1 e X2**: Sinalizadores luminosos / Lâmpadas. [CONCLUÍDO]  
    - Presets rápidos de atribuição A1-A2 e X1-X2. [CONCLUÍDO]  
  - **Identificação de Réguas de Bornes em Painéis (IEC 81346)**: Designadas pela letra **X**: [CONCLUÍDO]  
  - Tabela de subdivisão de funções de réguas de bornes: [CONCLUÍDO]  

| Identificação da Régua | Função Principal | Tipo de Circuito |
| :---- | :---- | :---- |
| XP (ou X0) | Bornes destinados à alimentação geral e motores | Potência / Força |
| XC | Bornes para interligação de botoeiras, sensores e lógicas | Comando / Auxiliar |
| XT | Bornes dedicados a transformadores e medição | Auxiliar |

  - Botões de classificação rápida da régua (-XP, -XC, -XT, -X1) e renumeração sequencial. [CONCLUÍDO]  
- **Bobinas com padrões especiais**: Seletor funcional de padrões (Padrão Instantânea, Ao Trabalho TON, Ao Repouso TOF, Cíclico/Intermitente, Retenção Biestável Latching) e ajuste de tempo de temporização. [CONCLUÍDO]  
- **Sinalizadores**: Apresentação de 6 cores industriais padronizadas (Verde, Vermelho, Amarelo, Azul, Branco, Laranja). [CONCLUÍDO]  
- **Sinalizadores Pulsantes**: Alternador de modo pulsante e campo editável para tempo de pulso em segundos. [CONCLUÍDO]  

## 4.2 Seletor: Referência Cruzada — [100% CONCLUÍDO]

- **Nenhum elemento selecionado**: Exibe a referência cruzada baseada em nomes de todo o projeto (busca/filtro, contagem total de tags e folhas, agrupamento hierárquico pai-contatos, instâncias, alertas de contatos órfãos / bobinas duplicadas, navegação "Ir" e resumo das réguas de bornes com renumeração global). [CONCLUÍDO]  
- **Elemento selecionado**: Exibe apenas as referências cruzadas do elemento selecionado, considerando nome, vínculo pai/irmãos com navegação "Ir", e ligação elétrica detalhada de bornes (em série, em paralelo e direta com badges coloridos e botões "Ir"), além das propriedades da régua de bornes e atalho para XRef Global. [CONCLUÍDO]

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