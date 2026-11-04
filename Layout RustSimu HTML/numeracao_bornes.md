# Identificação e numeração de bornes em aparelhos de comando e proteção

A identificação dos bornes (terminais) de contactores, relés, disjuntores e relés térmicos segue uma lógica normalizada, para que qualquer técnico leia o esquema e ligue o aparelho sem depender do fabricante.

## Normas de referência

- **IEC 60204-1**: equipamento elétrico de máquinas (identificação de condutores e aparelhos nos esquemas).
- **EN 50005**: regras gerais de marcação de terminais de contactos auxiliares.
- **EN 50011**: marcação de terminais de relés auxiliares.
- **EN 50012**: marcação de terminais de contactores e relés de comando.
- **IEC 60947-1 / 60947-5-1**: aparelhagem de baixa tensão e aparelhos de comando.
- **IEC 60947-4-1**: contactores, arrancadores e relés de sobrecarga (marcação 95-96 / 97-98).

## 1. Circuitos de potência (força)

Os polos principais usam um só algarismo (1 a 6). Os ímpares marcam a entrada (linha) e os pares a saída (carga).

| Polo | Entrada | Saída |
|---|---|---|
| 1 (L1) | 1 | 2 (T1 / U) |
| 2 (L2) | 3 | 4 (T2 / V) |
| 3 (L3) | 5 | 6 (T3 / W) |
| Neutro (4 polos) | 7 | 8 |

## 2. Contactos auxiliares (dois dígitos)

- **Dígito das dezenas (sequência):** identifica o contacto dentro do aparelho (1.º, 2.º, 3.º, ...). Os dois bornes do mesmo contacto partilham este dígito.
- **Dígito das unidades (função):** indica o tipo de contacto.

| Unidades | Função |
|---|---|
| 1-2 | NF (normalmente fechado) |
| 3-4 | NA (normalmente aberto) |
| 5-6 | NF com função especial (ex.: temporizado) |
| 7-8 | NA com função especial (ex.: temporizado) |

### Tabela por número de contacto

| Contacto n.º | Par NF | Par NA | Uso típico |
|---|---|---|---|
| 1 | 11/12 | 13/14 | Selo (13/14), encravamento (11/12) |
| 2 | 21/22 | 23/24 | Encravamento elétrico, feedback ao PLC |
| 3 | 31/32 | 33/34 | Sinalização, indicação de marcha |
| 4 | 41/42 | 43/44 | Segundo encravamento, lógica de sequência |

Blocos auxiliares adicionais continuam a sequência (53/54, 61/62, ...). Um bloco "22" tem 2 NA e 2 NF.

## 3. Bobinas e sinalização

- **A1 / A2**: terminais da bobina de contactores e relés (A1 normalmente positivo/fase, A2 comum/neutro).
- **X1 / X2** (ou H1 / H2): lâmpadas e sinalizadores. Designação de uso corrente; confirmar na documentação do fabricante.

## 4. Relés de sobrecarga (térmicos)

- **95-96**: contacto NF de disparo, em série com a bobina (A1-A2).
- **97-98**: contacto NA de sinalização (lâmpada de avaria, alarme, entrada de PLC).
- **Potência**: 1/L1, 3/L2, 5/L3 na entrada; 2/T1, 4/T2, 6/T3 na saída.

Não confundir 95/96 e 97/98 com os contactos auxiliares do contactor (13/14, 21/22).

## 5. Boas práticas

- Respeitar o tipo de contacto (NA ou NF) indicado pelos bornes.
- Usar a numeração do fabricante, que segue a norma.
- Representar os contactos na posição de repouso (bobina desenergizada).

## Resumo comparativo

| Tipo de circuito | Entrada | Saída | Exemplo |
|---|---|---|---|
| Potência L1 | 1 | 2 | Polo principal do contactor ou disjuntor |
| Potência L2 | 3 | 4 | Polo principal |
| Potência L3 | 5 | 6 | Polo principal |
| Auxiliar NA | x3 (13, 23...) | x4 (14, 24...) | Selo, botoneiras NA |
| Auxiliar NF | x1 (11, 21...) | x2 (12, 22...) | Encravamento, paragem |
| Especial NF / NA | x5 / x7 | x6 / x8 | Temporizados |
| Bobina | A1 | A2 | Contactores e relés |
| Térmico NF / NA | 95 / 97 | 96 / 98 | Corte do comando / sinalização de avaria |
| Sinalização | X1 | X2 | Lâmpadas e LEDs |
