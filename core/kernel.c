#include "vga.h"

// Função principal do kernel

/*
definimos a função principal do kernel, que é chamada a partir do código de boot.
Essa função é responsável por inicializar o sistema operacional e exibir uma mensagem de boas-vindas na tela do VGA.
Primeiro, a função chama clear_screen() para limpar a tela, e depois chama putstr() para exibir a mensagem "Bem-vindo ao YorUnix!".
*/
void kernel_main() {
    clear_screen(); // Limpa a tela do VGA 

    putstr("Bem-vindo ao YorUnix!"); // Exibe uma mensagem de boas-vindas
    while (1) {
        // Loop infinito para manter o kernel em execução
        __asm__("hlt"); // Instrução para colocar a CPU em estado de espera, economizando recursos
    }
}