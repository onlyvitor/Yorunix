#include "vga.h"

/*
Essas diretizes preprocessamento definem as constantes e macros
neste caso definimos o endereço de memória para a tela de texto do VGA,
as cores para o texto e o fundo, e as dimensões da tela, sendo BRANCO e VERDE.
Tais funções são essenciais para a implementação de um kernel básico,
permitindo que o sistema operacional exiba informações na tela durante a inicialização e operação.
*/

#define VGA_MEMORY (void *)0xB8000

#define WHITE_COLOR 0x0F
#define MAGENTA_COLOR 0x05
#define GREEN_COLOR 0x02
#define BLACK_COLOR 0x00
#define CYAN_COLOR 0x03
#define YELLOW_COLOR 0x0E

#define VGA_WIDTH 80 //dimensões da tela do VGA
#define VGA_HEIGHT 25 //dimensões da tela do VGA

// Função para limpar a tela do VGA

/*
definimos uma função para "limpar a tela",
que percorre toda a memória de vídeo do VGA e escreve um espaço em branco (representado por 0)
com a cor definida (WHITE_ON_BLACK) para cada posição da tela.
*/

void clear_screen() {
    char *vga_memo= VGA_MEMORY;
    unsigned int i = 0;

    while (i < VGA_WIDTH * VGA_HEIGHT) {
        vga_memo[i]= 0; // Escreve um espaço em branco
        vga_memo[i + 1]= WHITE_COLOR; // Define a cor do texto
        i += 2; // Move para o próximo caractere (2 bytes por caractere)
    }
}

// Função para imprimir uma string na tela do VGA

/*
definimos uma função para imprimir uma string na tela do VGA,
que recebe um ponteiro para a string e a posição (linha e coluna) onde a string deve ser exibida.
A função calcula o índice na memória de vídeo do VGA com base na linha e coluna fornecidas
e escreve cada caractere da string na posição correspondente, definindo a cor do texto como WHITE_COLOR.
*/

void putstr(const char *str){
    char *vga_memo= VGA_MEMORY;
    unsigned int i = 0;

    while (str[i] != '\0') {
        vga_memo[i * 2]= str[i]; // Escreve o caractere
        vga_memo[i * 2 + 1]= WHITE_COLOR; // Define a cor do texto
        i++;
    }
}