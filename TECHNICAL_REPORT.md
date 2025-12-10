# 📊 SysDash: Relatório Técnico & Documentação

---

## 1. Introdução

**SysDash** é um monitor de sistema de alta performance baseado em terminal (TUI - Terminal User Interface), desenvolvido para oferecer visibilidade em tempo real sobre o consumo de recursos (CPU, Memória, Disco, Rede e Processos) de servidores e estações de trabalho Linux.

Ao contrário de top/htop, o SysDash foca em uma experiência visual moderna e rica, priorizando a leitura rápida de informações críticas através de gráficos e *dashboards*, sem sacrificar a performance da máquina monitorada.

---

## 2. Decisões de Engenharia: Por que Rust? 🦀

A escolha da linguagem **Rust** não foi acidental. Para uma ferramenta de monitoramento de sistemas (System Monitor), os requisitos não-funcionais são estritos: o monitor não pode consumir os recursos que ele deve monitorar.

### 🛡️ Pilares da Escolha
1.  **Performance (Zero-Overhead)**: Rust compila para código de máquina nativo (via LLVM) e não possui *Garbage Collector*. Isso garante que o SysDash tenha uma pegada de memória minúscula e previsível, evitando os picos de CPU causados por GCs em linguagens como Go ou Java.
2.  **Memory Safety**: O sistema de *Ownership* do Rust garante, em tempo de compilação, que não haverá *Segmentation Faults* críticos, vazamentos de memória ou acesso indevido a dados liberados.
3.  **Fearless Concurrency**: A arquitetura do SysDash é multi-threaded. Em C++, isso seria uma fonte de *Data Races* perigosos. Em Rust, o compilador **recusa-se a compilar** código que compartilhe estado de forma insegura entre threads.

### 🎓 Análise Acadêmica (Critérios de LP)

Aplicando os critérios clássicos de avaliação de linguagens de programação ao projeto:

| Critério | Aplicação no SysDash |
| :--- | :--- |
| **Legibilidade** | O uso de **Pattern Matching** (`match`) em `app.rs` torna o fluxo de controle visualmente claro. Iteradores (`map`, `filter`) em `sys.rs` permitem descrever transformações de dados de forma declarativa e concisa. |
| **Confiabilidade** | A strictez do sistema de tipos e o tratamento obrigatório de erros com `Result` impedem que falhas de I/O ou estados inválidos (ex: enum `PopupState`) crashem o monitor silenciosamente. |
| **Custo** | Embora o **Custo de Aprendizado** e implementação seja maior (devido ao *borrow checker*), o **Custo de Execução** (recursos de hardware) e o **Custo de Manutenção** (correção de bugs futuros) são drasticamente reduzidos. |
| **Escrita** | As bibliotecas (`crates`) como `sysinfo` e `ratatui` oferecem **abstrações de custo zero**: codificamos em alto nível, mas a execução tem performance de baixo nível. |

---

## 3. Arquitetura do Sistema

O SysDash utiliza uma arquitetura **Multi-Threaded baseada em Atores Simplificados** para garantir que a Interface de Usuário (UI) nunca congele, mesmo se a leitura dos dados do sistema for lenta (ex: disco rígido lento).

### Fluxo de Dados

```ascii
+-----------------------+                            +-------------------------+
|   Thread Principal    |                            |     Thread Worker       |
|       (UI Loop)       |                            |    (Coleta de Dados)    |
|                       |      Command Channel       |                         |
|  [ Input Keyboard ] --+---> (mpsc::Sender) ------->| [ SystemMonitor ] <---+ |
|           |           |     "Matar Processo"       |         |             | |
|     (Renderiza)       |                            |     (Atualiza)        | |
|           |           |       State Channel        |         |             | |
|       [ TUI ] <-------+---- (mpsc::Receiver) <-----+ [ sysinfo Library ] --+ |
|                       |      "Novo Estado"         |                         |
+-----------------------+                            +-------------------------+
```

1.  **Separação de Responsabilidades**:
    *   **Frontend (Main Thread)**: Responsável apenas por desenhar na tela e capturar teclas. Mantém uma cópia somente leitura do último estado conhecido (`SystemState`).
    *   **Backend (Worker Thread)**: Roda em *loop* infinito. A cada segundo, coleta dados crus do Kernel (via `/proc`), calcula taxas e monta um snapshot (`SystemState`).

2.  **Comunicação (Canais)**:
    *   Nenhum `Mutex` de bloqueio global foi usado para o estado principal, evitando gargalos.
    *   A comunicação é feita via canais `std::sync::mpsc`: O Worker *envia* o estado completo para a UI, que *consome* e substitui seu estado local.

---

## 4. Anatomia do Código

O projeto é modularizado para facilitar manutenção:

### `src/main.rs` (O Maestro)
*   **Função**: Ponto de entrada.
*   **Responsabilidade**:
    *   Configura o terminal em modo *raw* (sem echo de teclas).
    *   Inicializa os canais de comunicação (`mpsc`).
    *   Spawna a thread de coleta (`spawn_system_worker`).
    *   Executa o loop principal de eventos, despachando inputs para o `App`.

### `src/app.rs` (O Cérebro)
*   **Função**: Gerenciamento de Estado da Aplicação.
*   **Responsabilidade**:
    *   Mantém o estado da UI: qual aba está selecionada, se há um popup aberto, o texto da busca.
    *   Interpreta teclas (`KeyEvent`): "Se apertar 'k', abra o popup de kill".
    *   Envia comandos para o Worker (ex: `SystemCommand::KillProcess`).

### `src/sys.rs` (O Motor)
*   **Função**: Camada de Dados e Hardware.
*   **Responsabilidade**:
    *   **`SystemMonitor`**: A struct que vive na thread worker. Detém a instância da lib `sysinfo`.
    *   **`SystemState`**: Um DTO (Data Transfer Object) simples, clonável, contendo apenas os dados prontos para exibição (vetores de processos, float de CPU).
    *   Realiza cálculos de derivadas (ex: taxa de download = bytes atuais - bytes anteriores).

### `src/ui.rs` (A Face)
*   **Função**: Renderização Visual.
*   **Responsabilidade**:
    *   Utiliza a biblioteca `ratatui` para desenhar Widgets.
    *   Define o Layout (chunks verticais/horizontais).
    *   Converte números brutos em representações visuais (Gráficos Sparkline, Barras de Progresso Coloridas).
    *   **Puramente Funcional**: Recebe o estado e desenha. Não altera dados.

---

## 5. Guia de Instalação e Execução

### Pré-requisitos
*   **Rust & Cargo**: [Instalar Rust](https://rustup.rs/)

### Rodando Localmente (Desenvolvimento)
Para compilar e rodar em modo debug (mais rápido de compilar, menos otimizado):
```bash
cargo run
```

### Gerando Binário de Produção (Release)
Para gerar um executável otimizado (menor e mais rápido):
```bash
cargo build --release
```
O binário estará disponível em: `./target/release/sysdash`

### Comandos da Ferramenta
*   `q` ou `Ctrl+C`: Sair.
*   `/`: Pesquisar processo por nome/PID.
*   `k`: Matar o processo selecionado (abre confirmação).
*   `s` ou `Tab`: Alternar ordenação (CPU / Memória / PID).
*   `?`: Ajuda.
