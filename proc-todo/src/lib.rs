use proc_macro::{token_stream::IntoIter, TokenStream, TokenTree};

fn read_arg(iter: &mut IntoIter) -> Option<TokenTree> {
    match iter.next() {
        Some(TokenTree::Punct(punct)) => {
            assert_eq!(punct.as_char(), ',');

            iter.next()
        }

        Some(other) => panic!("second item not Punct: {:?}", other),
        None => None,
    }
}

#[proc_macro]
pub fn test_mock_fn(args: TokenStream) -> TokenStream {
    let mut iter = args.into_iter();
    let name = match iter.next() {
        Some(TokenTree::Ident(ident)) => ident.to_string(),
        other => {
            panic!("arg 1 not ident: {:?}", other);
        }
    };

    let out = match read_arg(&mut iter) {
        Some(TokenTree::Literal(literal)) => {
            let args_num: usize = literal.to_string().parse().unwrap();

            let mut template_names = Vec::new();
            let mut arg_pairs = Vec::new();

            for i in 0..args_num {
                let template_name = format!("T{i}");
                template_names.push(template_name.clone());
                arg_pairs.push(format!("arg{i}: {template_name}"));
            }
            let template = if template_names.is_empty() {
                String::new()
            } else {
                format!("<{}>", template_names.join(", "))
            };
            let args = if arg_pairs.is_empty() {
                String::new()
            } else {
                arg_pairs.join(", ")
            };

            match read_arg(&mut iter) {
                Some(TokenTree::Ident(ident)) => {
                    let return_type = ident.to_string();
                    format!(
                        "pub unsafe extern \"C\" fn {name}{template}({args}) -> {return_type} {{ unimplemented!(\"{name}\") }}"
                    )
                }
                Some(other) => panic!("arg 3 not Ident: {:?}", other),
                None => format!("pub unsafe extern \"C\" fn {name}{template}({args}) {{}}"),
            }
        }

        Some(other) => panic!("arg 2 not Literal: {:?}", other),
        None => format!("pub unsafe extern \"C\" fn {name}() {{}}"),
    };

    eprintln!("{}", out);

    out.parse().unwrap()
}
