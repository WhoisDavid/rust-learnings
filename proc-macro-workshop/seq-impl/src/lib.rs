extern crate proc_macro;

use proc_macro2::{TokenStream, TokenTree};
use proc_macro_hack::proc_macro_hack;
use syn::{braced, parse_macro_input, Token};

#[proc_macro]
pub fn seq(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let mut seq_input = parse_macro_input!(input as Seq);
    // let out = range.map(|i| replace_number(&name, i, body));
    seq_input.expand().into()
}

#[proc_macro_hack]
pub fn eseq(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    seq(input)
}

struct Seq {
    name: syn::Ident,
    range: std::ops::Range<u64>,
    body: proc_macro2::TokenStream,
    repeated_section: bool,
}

impl syn::parse::Parse for Seq {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let name: syn::Ident = input.parse()?;
        let _: Token![in] = input.parse()?;

        // Range parsing and definition
        let start: u64 = input.parse::<syn::LitInt>()?.base10_parse()?;
        let _: Token![..] = input.parse()?;
        let inclusive = input.parse::<Token![=]>().is_ok();
        let end: u64 = input.parse::<syn::LitInt>()?.base10_parse()?;
        let range = if inclusive {
            start..(end + 1)
        } else {
            start..end
        };

        // Parse content between braces as a TokenStream
        let content;
        let _ = braced!(content in input);
        let body: proc_macro2::TokenStream = content.parse()?;

        Ok(Seq::new(name, range, body))
    }
}

impl Seq {
    fn new(name: syn::Ident, range: std::ops::Range<u64>, body: proc_macro2::TokenStream) -> Self {
        Self {
            name,
            range,
            body,
            repeated_section: false,
        }
    }

    // Performs the whole expansion
    fn expand(&mut self) -> TokenStream {
        let token_stream = self.replace_repeated_sections(self.body.clone());
        if self.repeated_section {
            return token_stream;
        }
        self.range.clone().fold(TokenStream::new(), |mut ts, val| {
            let ts_aux = self.replace_number(val, self.body.clone());
            ts.extend(ts_aux);
            ts
        })
    }

    // Replacement of repeated sections `#(..)*`
    fn replace_repeated_sections(&mut self, body: TokenStream) -> TokenStream {
        let mut output_stream = TokenStream::new();
        let mut token_iter = body.into_iter();
        // Always keep tokenstream_peek_ahead one further
        while let Some(token) = token_iter.next() {
            let output_token = match token {
                // Expand group recursively
                TokenTree::Group(ref group) => {
                    let del = group.delimiter();
                    let stream = self.replace_repeated_sections(group.stream());
                    let mut group = proc_macro2::Group::new(del, stream);
                    group.set_span(token.span());
                    TokenTree::from(group)
                }
                // Look for repeated section #(..)* = [Punct(#), Group(delimiter=Parenthesis), Punct(*)]
                TokenTree::Punct(ref punct) if punct.as_char() == '#' => {
                    match peek_next_two(&token_iter) {
                        (Some(TokenTree::Group(group)), Some(TokenTree::Punct(punct)))
                            if group.delimiter() == proc_macro2::Delimiter::Parenthesis
                                && punct.as_char() == '*' =>
                        {
                            self.repeated_section = true;
                            // Advance iterator
                            token_iter.next(); // Group()
                            token_iter.next(); // Punct(*)

                            let del = proc_macro2::Delimiter::None;
                            let stream =
                                self.range.clone().fold(TokenStream::new(), |mut ts, val| {
                                    let ts_aux = self.replace_number(val, group.stream());
                                    ts.extend(ts_aux);
                                    ts
                                });
                            let mut group = proc_macro2::Group::new(del, stream);
                            group.set_span(token.span());
                            TokenTree::from(group)
                        }
                        _ => token,
                    }
                }
                // Otherwise just return the token as-is
                _ => token,
            };
            output_stream.extend(TokenStream::from(output_token));
        }

        output_stream
    }

    // Replacement of the seq variable with a given value `val`
    fn replace_number(&self, val: u64, body: TokenStream) -> TokenStream {
        let mut output_stream = TokenStream::new();
        let mut token_iter = body.into_iter();
        // Always keep tokenstream_peek_ahead one further
        while let Some(token) = token_iter.next() {
            let current_span = token.span();
            let output_token = match token {
                // N
                TokenTree::Ident(ref ident) if ident == &self.name => {
                    let mut lit = proc_macro2::Literal::u64_unsuffixed(val);
                    lit.set_span(token.span());
                    TokenTree::from(lit)
                }
                // Ident, check for "ident#N" = [Ident(..), Punct(#), Ident(N)]
                TokenTree::Ident(ref prefix) => {
                    match peek_next_two(&token_iter) {
                        (Some(TokenTree::Punct(punct)), Some(TokenTree::Ident(ref ident)))
                            if punct.as_char() == '#' && ident == &self.name =>
                        {
                            // Advance iterator
                            token_iter.next(); // Punct(#)
                            token_iter.next(); // Ident(name)
                            let concat = format!("{}{}", prefix, val);
                            let concat_ident = proc_macro2::Ident::new(&concat, current_span);
                            TokenTree::from(concat_ident)
                        }
                        _ => token,
                    }
                }
                // Expand group recursively
                TokenTree::Group(ref group) => {
                    let del = group.delimiter();
                    let stream = self.replace_number(val, group.stream());
                    let mut group = proc_macro2::Group::new(del, stream);
                    group.set_span(token.span());
                    TokenTree::from(group)
                }
                // Otherwise just return the token as-is
                _ => token,
            };
            output_stream.extend(TokenStream::from(output_token));
        }

        output_stream
    }
}

// Helper to peek at the two next elements of an token stream iterator
fn peek_next_two(
    token_iter: &proc_macro2::token_stream::IntoIter,
) -> (Option<TokenTree>, Option<TokenTree>) {
    // Should also be doable with itertools::MultiPeek instead
    let mut peek = token_iter.clone();
    (peek.next(), peek.next())
}
