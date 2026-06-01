use std::io;
use std::thread;
use std::time::Duration;
use mysql::prelude::*;
use mysql::*;

const INITIAL_ADMIN_PASSWORD: &str = "Un1f3c@f";

#[derive(Debug)]
struct Produto {
    pro_id: i32,
    pro_nome: String,
    prod_desc: String,
    prod_preco: f64,
    qntd_estoque: i32,
    cat_id: i32,
    forn_id: i32,
}

fn main() {
    unsafe { std::env::set_var("RUST_BACKTRACE", "1"); }
    let mut conn = conectar_banco();
    if !usuario_existe(&mut conn) {
        criar_primeiro_usuario(&mut conn);
    }

    loop {
        let (usuario_id, adm) = loop {
            if let Some((id, adm)) = login_cli(&mut conn) {
                break (id as u32, adm);
            }
        };

        loop {
            if adm {
                println!("Escolha: \n1. Listar\n2. Adicionar\n3. Atualizar\n4. Remover\n5. Criar usuário\n6. Comprar\n7. Abrir jogo\n8. Logout");
            } else {
                println!("Escolha: \n1. Listar\n2. Comprar\n3. Abrir jogo\n4. Logout");
            }

            let mut choice = String::new();
            io::stdin().read_line(&mut choice).unwrap();
            let choice: i32 = choice.trim().parse().unwrap_or(0);

            match choice {
                1 => listar_produtos(&mut conn),
                2 if adm => adicionar_produto_cli(&mut conn),
                3 if adm => atualizar_produto_cli(&mut conn),
                4 if adm => remover_produto_cli(&mut conn),
                5 if adm => criar_usuario_cli(&mut conn),
                6 if adm => comprar_jogo_cli(&mut conn, usuario_id),
                7 if adm => abrir_jogo_cli(&mut conn, usuario_id),
                8 if adm => break,
                2 if !adm => comprar_jogo_cli(&mut conn, usuario_id),
                3 if !adm => abrir_jogo_cli(&mut conn, usuario_id),
                4 if !adm => break,
                _ => println!("Escolha inválida"),
            }
        }
    }
}

fn conectar_banco() -> PooledConn {
    let url = "mysql://root:root@localhost:3306/Loja_Jogos";
    let pool = match Pool::new(url) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Erro detalhado ao conectar no banco: {}", e);
            std::process::exit(1);
        }
    };
    
    match pool.get_conn() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Erro ao obter conexão do pool: {}", e);
            std::process::exit(1);
        }
    }
}

fn usuario_existe(conn: &mut PooledConn) -> bool {
    conn.exec_first::<u64, _, _>("SELECT COUNT(*) FROM usuarios", ())
        .ok()
        .flatten()
        .unwrap_or(0)
        > 0
}

fn criar_primeiro_usuario(conn: &mut PooledConn) {
    println!("Nenhum usuário encontrado. Digite a senha mestra para criar o primeiro usuário:");
    let mut senha = String::new();
    io::stdin().read_line(&mut senha).unwrap();
    if senha.trim() != INITIAL_ADMIN_PASSWORD {
        println!("Senha mestra incorreta.");
        std::process::exit(1);
    }

    println!("Login do primeiro usuário admin:");
    let mut login = String::new();
    io::stdin().read_line(&mut login).unwrap();
    println!("Senha do primeiro usuário admin:");
    let mut senha_usuario = String::new();
    io::stdin().read_line(&mut senha_usuario).unwrap();

    let login = login.trim();
    let senha_usuario = senha_usuario.trim();
    if login.is_empty() || senha_usuario.is_empty() {
        println!("Dados inválidos.");
        std::process::exit(1);
    }

    if criar_usuario_db(conn, login, senha_usuario, true) {
        println!("Administrador criado com sucesso.");
    } else {
        println!("Erro ao criar administrador.");
        std::process::exit(1);
    }
}

fn login_cli(conn: &mut PooledConn) -> Option<(i32, bool)> {
    println!("Login:");
    let mut login = String::new();
    io::stdin().read_line(&mut login).unwrap();
    println!("Senha:");
    let mut senha = String::new();
    io::stdin().read_line(&mut senha).unwrap();

    let login = login.trim();
    let senha = senha.trim();
    if login.is_empty() || senha.is_empty() {
        println!("Login ou senha inválidos.");
        return None;
    }

    if let Some((id, adm)) = validar_login(conn, login, senha) {
        Some((id, adm))
    } else {
        println!("Login ou senha incorretos.");
        None
    }
}

fn validar_login(conn: &mut PooledConn, login: &str, senha: &str) -> Option<(i32, bool)> {
    conn.exec_first("SELECT CAST(ID AS SIGNED), admin FROM usuarios WHERE login = ? AND senha = ?", (login, senha))
        .ok()
        .flatten()
}

fn criar_usuario_db(conn: &mut PooledConn, login: &str, senha: &str, adm: bool) -> bool {
    conn.exec_drop(
        "INSERT INTO usuarios (login, senha, admin, dinheiro) VALUES (?, ?, ?, 1000)",
        (login, senha, adm),
    )
    .is_ok()
}

fn criar_usuario_cli(conn: &mut PooledConn) {
    println!("Novo login:");
    let mut login = String::new();
    io::stdin().read_line(&mut login).unwrap();
    println!("Nova senha:");
    let mut senha = String::new();
    io::stdin().read_line(&mut senha).unwrap();
    println!("Informe 1 para administrador ou 2 para usuário comum:");
    let mut adm = String::new();
    io::stdin().read_line(&mut adm).unwrap();

    let login = login.trim();
    let senha = senha.trim();
    let adm = adm.trim().parse::<i32>().unwrap_or(2) == 1;

    if login.is_empty() || senha.is_empty() {
        println!("Dados inválidos.");
        return;
    }

    if criar_usuario_db(conn, login, senha, adm) {
        println!("Usuário criado com sucesso.");
    } else {
        println!("Erro ao criar usuário.");
    }
}

fn obter_saldo(conn: &mut PooledConn, usuario_id: u32) -> Option<f64> {
    conn.exec_first("SELECT dinheiro FROM usuarios WHERE CAST(ID AS SIGNED) = ?", (usuario_id as i32,))
        .ok()
        .flatten()
}

fn atualizar_saldo(conn: &mut PooledConn, usuario_id: u32, valor: f64) -> bool {
    conn.exec_drop(
        "UPDATE usuarios SET dinheiro = dinheiro + ? WHERE CAST(ID AS SIGNED) = ?",
        (valor, usuario_id as i32),
    )
    .is_ok()
}

fn listar_produtos_compra(conn: &mut PooledConn) -> Vec<Produto> {
    conn.query_map(
        "SELECT pro_id, pro_nome, prod_desc, prod_preco, qntd_estoque, cat_id, forn_id FROM tb_produtos WHERE qntd_estoque > 0 ORDER BY pro_nome",
        |(pro_id, pro_nome, prod_desc, prod_preco, qntd_estoque, cat_id, forn_id)| Produto { pro_id, pro_nome, prod_desc, prod_preco, qntd_estoque, cat_id, forn_id },
    )
    .unwrap_or_else(|e| {
        eprintln!("Erro ao consultar produtos: {}", e);
        Vec::new()
    })
}

fn adicionar_biblioteca(conn: &mut PooledConn, usuario_id: u32, produto_id: i32) -> bool {
    conn.exec_drop(
        "INSERT INTO biblioteca (usuario_id, produto_id) VALUES (?, ?)",
        (usuario_id as i32, produto_id),
    )
    .is_ok()
}

fn produto_na_biblioteca(conn: &mut PooledConn, usuario_id: u32, produto_id: i32) -> bool {
    conn.exec_first::<u64, _, _>(
        "SELECT COUNT(*) FROM biblioteca WHERE CAST(usuario_id AS SIGNED) = ? AND produto_id = ?",
        (usuario_id as i32, produto_id),
    )
    .ok()
    .flatten()
    .unwrap_or(0)
        > 0
}

fn comprar_jogo_cli(conn: &mut PooledConn, usuario_id: u32) {
    let saldo_atual = obter_saldo(conn, usuario_id).unwrap_or(0.0);
    println!("Seus créditos: R${:.2}", saldo_atual);
    
    let produtos = listar_produtos_compra(conn);

    if produtos.is_empty() {
        println!("Nenhum jogo disponível para compra.");
        return;
    }

    println!("Jogos disponíveis:");
    for (idx, p) in produtos.iter().enumerate() {
        println!("{}. {} - R${:.2}", idx + 1, p.pro_nome, p.prod_preco);
    }

    println!("Escolha o número do jogo (0 para cancelar):");
    let mut choice = String::new();
    io::stdin().read_line(&mut choice).unwrap();
    let choice: usize = choice.trim().parse().unwrap_or(0);

    if choice == 0 || choice > produtos.len() {
        println!("Cancelado.");
        return;
    }

    let produto = &produtos[choice - 1];

    if produto_na_biblioteca(conn, usuario_id, produto.pro_id) {
        println!("Você já possui este jogo.");
        return;
    }

    if let Some(saldo) = obter_saldo(conn, usuario_id) {
        if saldo < produto.prod_preco {
            println!("Saldo insuficiente. Você tem R${:.2}", saldo);
            return;
        }

        if atualizar_saldo(conn, usuario_id, -produto.prod_preco) && adicionar_biblioteca(conn, usuario_id, produto.pro_id) {
            conn.exec_drop(
                "UPDATE tb_produtos SET qntd_estoque = qntd_estoque - 1 WHERE pro_id = ?",
                (produto.pro_id,),
            ).ok();
            println!("Jogo comprado com sucesso! Novo saldo: R${:.2}", saldo - produto.prod_preco);
        } else {
            println!("Erro ao completar a compra.");
        }
    } else {
        println!("Erro ao obter saldo.");
    }
}

fn abrir_jogo_cli(conn: &mut PooledConn, usuario_id: u32) {
    let jogos: Vec<(i32, String)> = conn.exec_map(
        "SELECT p.pro_id, p.pro_nome FROM biblioteca b JOIN tb_produtos p ON b.produto_id = p.pro_id WHERE CAST(b.usuario_id AS SIGNED) = ? ORDER BY p.pro_nome",
        (usuario_id as i32,),
        |(pro_id, pro_nome)| (pro_id, pro_nome),
    )
    .unwrap_or_else(|e| {
        eprintln!("Erro ao consultar biblioteca: {}", e);
        Vec::new()
    });

    if jogos.is_empty() {
        println!("Você não possui nenhum jogo.");
        return;
    }

    println!("Seus jogos:");
    for (idx, (_, nome)) in jogos.iter().enumerate() {
        println!("{}. {}", idx + 1, nome);
    }

    println!("Escolha o número do jogo (0 para cancelar):");
    let mut choice = String::new();
    io::stdin().read_line(&mut choice).unwrap();
    let choice: usize = choice.trim().parse().unwrap_or(0);

    if choice == 0 || choice > jogos.len() {
        println!("Cancelado.");
        return;
    }

    let (_, nome_jogo) = &jogos[choice - 1];
    
    println!("Abrindo: {}", nome_jogo);
    println!("Pressione Ctrl+C para sair do jogo.");

    let ganho_por_intervalo = 10.0;
    let mut ganho_total = 0.0;

    loop {
        thread::sleep(Duration::from_secs(5));
        ganho_total += ganho_por_intervalo;
        atualizar_saldo(conn, usuario_id, ganho_por_intervalo);
        
        let saldo_atual = obter_saldo(conn, usuario_id).unwrap_or(0.0);
        println!("[{}] Ganho total: R${:.2} | Saldo atual: R${:.2}", nome_jogo, ganho_total, saldo_atual);
    }
}

fn listar_produtos(conn: &mut PooledConn) {
    let produtos: Vec<Produto> = conn
        .query_map(
            "SELECT pro_id, pro_nome, prod_desc, prod_preco, qntd_estoque, cat_id, forn_id FROM tb_produtos ORDER BY pro_nome",
            |(pro_id, pro_nome, prod_desc, prod_preco, qntd_estoque, cat_id, forn_id)| Produto { pro_id, pro_nome, prod_desc, prod_preco, qntd_estoque, cat_id, forn_id },
        )
        .unwrap_or_else(|e| {
            eprintln!("Erro ao consultar produtos: {}", e);
            Vec::new()
        });

    if produtos.is_empty() {
        println!("Nenhum produto encontrado.");
        return;
    }

    for p in produtos {
        println!("ID: {} | {} | R${:.2} | Estoque: {}\n{}\n", p.pro_id, p.pro_nome, p.prod_preco, p.qntd_estoque, p.prod_desc);
    }
}

fn adicionar_produto_db(conn: &mut PooledConn, nome: &str, desc: &str, preco: f64, estoque: i32) -> bool {
    let cat_id = 1;
    let forn_id = 1;
    conn.exec_drop(
        "INSERT INTO tb_produtos (pro_nome, prod_desc, prod_preco, qntd_estoque, cat_id, forn_id) VALUES (?, ?, ?, ?, ?, ?)",
        (nome, desc, preco, estoque, cat_id, forn_id),
    )
    .is_ok()
}

fn atualizar_produto_db(conn: &mut PooledConn, pro_id: i32, nome: &str, desc: &str, preco: f64, estoque: i32) -> bool {
    conn.exec_drop(
        "UPDATE tb_produtos SET pro_nome = ?, prod_desc = ?, prod_preco = ?, qntd_estoque = ? WHERE pro_id = ?",
        (nome, desc, preco, estoque, pro_id),
    )
    .is_ok()
}

fn remover_produto_db(conn: &mut PooledConn, pro_id: i32) -> bool {
    conn.exec_drop("DELETE FROM tb_produtos WHERE pro_id = ?", (pro_id,)).is_ok()
}

fn adicionar_produto_cli(conn: &mut PooledConn) {
    let mut nome = String::new();
    let mut desc = String::new();
    let mut preco = String::new();
    let mut estoque = String::new();

    println!("Nome:");
    io::stdin().read_line(&mut nome).unwrap();
    println!("Descrição:");
    io::stdin().read_line(&mut desc).unwrap();
    println!("Preço:");
    io::stdin().read_line(&mut preco).unwrap();
    println!("Quantidade em estoque:");
    io::stdin().read_line(&mut estoque).unwrap();

    let nome = nome.trim();
    let desc = desc.trim();
    let preco: f64 = preco.trim().parse().unwrap_or(0.0);
    let estoque: i32 = estoque.trim().parse().unwrap_or(0);

    if nome.is_empty() {
        println!("Nome obrigatório.");
        return;
    }

    if adicionar_produto_db(conn, nome, desc, preco, estoque) {
        println!("Jogo adicionado com sucesso!");
    } else {
        println!("Erro ao adicionar jogo.");
    }
}

fn atualizar_produto_cli(conn: &mut PooledConn) {
    println!("ID para atualizar:");
    let mut id = String::new();
    io::stdin().read_line(&mut id).unwrap();
    let id: i32 = id.trim().parse().unwrap_or(0);
    if id == 0 {
        println!("ID inválido.");
        return;
    }

    println!("Novo nome:");
    let mut nome = String::new();
    io::stdin().read_line(&mut nome).unwrap();
    println!("Nova descrição:");
    let mut desc = String::new();
    io::stdin().read_line(&mut desc).unwrap();
    println!("Novo preço:");
    let mut preco = String::new();
    io::stdin().read_line(&mut preco).unwrap();
    println!("Nova quantidade:");
    let mut estoque = String::new();
    io::stdin().read_line(&mut estoque).unwrap();

    let nome = nome.trim();
    let desc = desc.trim();
    let preco: f64 = preco.trim().parse().unwrap_or(0.0);
    let estoque: i32 = estoque.trim().parse().unwrap_or(0);

    if nome.is_empty() {
        println!("Nome obrigatório.");
        return;
    }

    if atualizar_produto_db(conn, id, nome, desc, preco, estoque) {
        println!("Jogo atualizado com sucesso!");
    } else {
        println!("Erro ao atualizar jogo.");
    }
}

fn remover_produto_cli(conn: &mut PooledConn) {
    println!("ID do produto para remover:");
    let mut id = String::new();
    io::stdin().read_line(&mut id).unwrap();
    let id: i32 = id.trim().parse().unwrap_or(0);
    if id == 0 {
        println!("ID inválido.");
        return;
    }

    if remover_produto_db(conn, id) {
        println!("Jogo removido com sucesso.");
    } else {
        println!("Erro ao remover jogo.");
    }
}
