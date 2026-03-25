#ifndef VGA_H
#define VGA_H

// Definições para o driver de vídeo VGA
// Este arquivo contém as definições e funções para interagir com a memória de vídeo do VGA

void clear_screen(); // Função para limpar a tela do VGA
void putstr(const char *str); // Função para imprimir uma string na tela do VGA

#endif // VGA_H