Modificações. Tudo o que estiver entre parênteses é um comentário para você, não deve entrar no projeto.

Vamos começar pela barra de menus que está faltando, esta deve conter:

# BARRA DE MENUS

## Arquivo

- Últimos Arquivos  
- Novo	Ctrl \+ N  
- Abrir	Ctrl \+ O  
- Salvar	Ctrl \+ S 	(caso seja a primeira vez que o projeto seja salvo, vai perguntar o formato, por defeito vai salvar em JSON)  
- Salvar Como	Ctrl \+ Shift \+ S 	(comportamento padrão de “salvar como” com os formatos que utilizamos: JSON, TOML, CAD, QET)  
- Salvar Cópia	Ctrl \+ Alt \+ S  
- Fechar 	Ctrl \+ W  
- Exportar	Ctrl \+ X	(PNG, JPG, BMP, SVG, DXF, PDF) (Toda página ou seleção) (Voltaremos a este item no futuro)  
- Imprimir	Ctrl \+ P

## Editar

- Desfazer Ctrl \+ Z  
- Refazer	Ctrl \+ Shift \+ Z  
- Cortar	Ctrl \+ X  
- Copiar	Ctrl \+ C  
- Colar	Ctrl \+ V (Cola no ponteiro do mouse, confirma o local com clique)  
- Colar na Posição Original	(Ctrl \+ Shift \+ V)  
- Duplicar	Ctrl \+ D  
- Inserir o Último Elemento	A  
- Colorir Elemento (Abre janela de seleção de cor para o elemento, troca o preto no caso de tema light ou o branco no caso de tema dark pela cor escolhida)  
- **Adicionar**  
  - Adicionar Texto em Linha  
  - Adicionar Campo de Texto  
  - Adicionar Imagem  
  - Adicionar Linha  
  - Adicionar Retângulo  
  - Adicionar Círculo  
  - Adicionar Arco  
  - (Teremos mais itens aqui no futuro)  
- **Seleção**  
  - Selecionar Tudo	Ctrl \+ A  
  - Anular a Seleção	Ctrl \+ Shift \+ A  
  - Inverter Seleção	Ctrl \+ I  
- Copiar informação do Elemento	Ctrl \+ Alt \+ C  
- Colar Informação do Elemento	Ctrl \+ Alt \+ V  
- Deletar	Del  
- Rotacionar	R	(Incrementos de 90º; 0º, 90º, 180º, 270º)  
- Rotacionar como Grupo	Shift \+ R  
- Espelhar Horizontalmente	M  
- Espelhar Verticalmente	F  
- **Agrupar**  
  - Agrupar	Ctrl \+ G  
  - Desagrupar	Ctrl \+ U  
- **Linhas e Colunas**  
  - Adicionar uma Linha  
  - Apagar uma Linha  
  - Adicionar uma Coluna  
  - Apagar uma Coluna  
- Propriedades da Página	Ctrl \+ Shift \+ Enter

## Projeto

- Propriedades do Projeto (Veremos isso depois)  
- Criação Automática de Condutores (Quando os bornes estão alinhados vertical ou horizontalmente os condutores são traçados e conectados ao elementos automaticamente, ligado por defeito)  
- Desconexão Automática de Condutores (Quando o elemento é movido o condutor fica estático e não acompanha o mesmo desligando dos bornes, desligado por defeito)  
- Adicionar uma Página	Ctrl \+ T  
- Deletar esta Página	Ctrl \+ Shift \+ BackSpace  
- (Decidiremos depois como fazer a exportação do projeto, exemplo CSV, numeração de conectores, BOM, etc)

## Display

- Mostrar projetos  
- Selecionar (Habilita/Desabilita a Seleção de Elementos, Ligado por defeito, não pode ser ligado em conjunto com Mover)  
- Mover (Move o Canvas de Trabalho de forma livre com o botão direito do mouse apertado)  
- Mostrar a grelha  
- Ajustar à grelha Ctrl+Shift+G  
- Passo da grelha  
- Mira em todo o fólio  
- Grelha de texto  
- Ver os guias  
- Manter cabeçalhos visíveis  
- Mostrar limites das células  
- Mostrar  
- Cor de fundo da folha  
- Ampliar (Zoom In) Ctrl \+ \+  
- Reduzir (Zoom Out) Ctrl \+ \-  
- Zoom do conteúdo Ctrl \+ 8  
- Ajustar à vista Ctrl \+ 9  
- Repor zoom Ctrl \+ 0  
- Centrar no cursor

*Ainda tem os Menus Settings, Window e Help*

Agora vamos a TOOLBAR, apenas ícones, texto aparece com hover do mouse:

# TOOLBAR 

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
- Mostrar a Grelha (Ligado por Defeito)  
- Snap à Grelha (Ligado por Defeito)  
- (Dropdown de seleção de densidade de grelha)  
  - Fina (10 pts por cm)  
  - Média (5 pts por cm)  
  - Larga (2.5 pts por cm)  
- (Dropdown de Textos)  
  - Ligado  
  - Desligado  
- Modo Claro, Escuro e Sistema

# ELEMENTOS E PROJETOS

Barra da esquerda Coleções/Projetos deve mudar o nome para Elementos e Projetos.  
Na parte de baixo deve ter dois seletores, o primeiro Elementos e o segundo Projetos.   
O seletor Elementos deve estar selecionado por defeito. Quando selecionado o seletor Elementos a barra deve apresentar o comportamento que está agora, com os símbolos os quais usamos. Um duplo clique ao nome do símbolo, o arrastar do nome do símbolo para o Canvas de Trabalho ou o clique em Inserir no Esquema deve inserir o Símbolo ao Canvas de Trabalho. O duplo clique ou o clique em Inserir no Esquema deve habilitar o símbolo ao clique e no próximo clique ele deve ser posto no seu devido lugar. Caso o utilizador clique como botão direito ou clique em ESC a operação deverá ser desfeita e o ponteiro do mouse voltar ao modo de seleção. Ao arrastar o Símbolo para o Canvas de Trabalho o Símbolo é simplesmente posto em seu lugar.  
Quando selecionado o seletor Projetos a barra deve mostrar os últimos projetos trabalhados pelo utilizador, o utilizador pode abrir o projeto com um duplo clique ou no botão abrir. O utilizador pode apagar o projeto com a tecla delete ou o botão apagar, esta opção abre uma janela de confirmação para a deleção do projeto ao ser apagado. Os projetos devem mostrar um menu dropdown com suas páginas ficando: Nome do Projeto \-\> (Número da página/Quantidade de páginas do projeto) Nome da página  
A barra deve sempre apresentar um ícone chevron para Colapso, ao ser colapsada o ícone deve ficar disponível para a barra ser restaurada.

# PROPRIEDADE DO ELEMENTO

Barra da direita Propriedade do Elemento deve continuar com o mesmo nome. Deve mostrar 3 seletores: Propriedades (que atualmente chama-se informações) e Referência Cruzada.  
Propriedades:  
Se algum elemento estiver selecionado: deve mostrar as propriedades editáveis do elemento selecionado:

- Nome:  
- Função:  
- Borne N (N deverá ser o cada borne que o elemento tiver, seguindo o padrão da norma IEC 60947-1 em conjunto com a IEC 81346 para a designação estruturada de sistemas e réguas de bornes:  
  -   
  - 1\. Circuitos de Potência (Força)  
  -   
  - Nos elementos de potência (como contatores, disjuntores-motores e relés térmicos), a numeração é composta por um único dígito:  
  - • Números Ímpares (1, 3, 5\) ou (L1, L2, L3): Indicam os bornes de entrada (alimentação das fases).  
  - • Números Pares (2, 4, 6\) ou (T1, T2, T3): Indicam os bornes de saída (linha que vai para a carga/motor).  
  -   
  - 2\. Circuitos de Comando  
  -   
  - Nos contactos auxiliares de comando, os bornes utilizam dois dígitos:  
  - • Primeiro dígito (Número de sequência): Indica a ordem do contacto no aparelho (1º contacto, 2º contacto, etc.).  
  - • Segundo dígito (Função do contacto): Define o comportamento elétrico do contacto:  
  - 	• 1 e 2: Contactos Normalmente Fechados (NF / NC) (Ex: 11-12, 21-22).  
  - 	• 3 e 4: Contactos Normalmente Abertos (NA / NO) (Ex: 13-14, 23-24).  
  - 	• 5 e 6: Contactos NF de dispositivos especiais, como temporizadores ou relés térmicos de sobrecarga.  
  - 	• 7 e 8: Contactos NA de dispositivos especiais (Ex: 97-98 no relé térmico para sinalização de falha).  
  -   
  - 3\. Bobinas e Sinalizadores  
  -   
  - • Bobinas de comando (Contatores/Relés): São identificadas por A1 e A2.  
  - • Sinalizadores luminosos/Lâmpadas: São identificados por X1 e X2.  
  - Identificação de Réguas de Bornes (Painéis Elétricos)  
  - De acordo com as boas práticas derivadas da norma IEC 81346, as réguas de bornes de passagem nos painéis são designadas pela letra X, frequentemente subdivididas para separar as funções:

| Identificação da Régua | Função Principal | Tipo de Circuito |
| :---- | :---- | :---- |
| XP (ou X0) | Bornes destinados à alimentação geral e motores | Potência / Força |
| XC | Bornes para interligação de botoeiras, sensores e lógicas | Comando / Auxiliar |
| XT | Bornes dedicados a transformadores e medição | Auxiliar |

- Bobinas com padrões especiais devem apresentar esses padrões para edição.  
- Sinalizadores devem apresentar a cor para escolher  
- Sinalizadores Pulsantes devem escolher o tempo de pulso

Referência Cruzada: Se nenhum elemento estiver selecionado deve mostrar a referência cruzada baseada em nomes do projeto completo assim como faz atualmente. Se algum elemento estiver selecionado deve mostrar apenas as referências cruzadas deste elemento, baseada em nome e em ligação de bornes, tanto em série quanto em paralelo.

# JANELA PRINCIPAL

Na Janela Principal, Canvas de Trabalho, teremos uma barra de título, na primeira linha desta barra de título temos o nome do Esquema aberto centralizado, caso o esquema tenha sido modificado e não salvo este ganha o marcador \[Modificado\]. No canto direito  desta primeira linha temos três ícones em quadrados: Novo, Abrir e Fechar, estes ícones têm o mesmo tamanho da linha. Na segunda linha temos Abas com o número da página seguido pelo nome da página. O nome da página se dado duplo clique fica selecionado e pode ser editado. Com o clique do botão direito abre as opções de editar o nome da página, editar o número da página (caso haja mais de uma), deletar a página (aviso que o projeto será fechado e oferecer para salvar caso haja apenas uma página), criar uma página antes e criar uma página depois. No canto do lado esquerdo desta linha um ícone para criar uma página antes, no canto direito um ícone para criar uma página depois.