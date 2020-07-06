extern crate proc_macro;

use proc_macro::TokenStream;
use quote::quote;
use syn::parse_macro_input;
use syn::visit_mut::VisitMut;
use syn::Item;

#[proc_macro_attribute]
pub fn sorted(args: TokenStream, input: TokenStream) -> TokenStream {
    let _ = args;
    let mut output = input.clone();
    let item = parse_macro_input!(input as syn::Item);

    // Add the potential error to the output stream
    if let Err(err) = sorted_impl(&item) {
        output.extend::<TokenStream>(err.to_compile_error().into());
    }
    output
}

// Helper to implement the sorted macro
// - Check if the item is an Enum
// - Build a vector of variants' ident
// - Check the order based on the ident's String representation
fn sorted_impl(item: &Item) -> Result<(), syn::Error> {
    // Make sure #[sorted] is applied to an enum
    let item_enum = is_enum(item)?;

    // Create a vector of variants' ident
    let variants_ident = item_enum
        .variants
        .iter()
        .map(|v| &v.ident)
        .collect::<Vec<_>>();

    // Check the order and return error if present
    check_order(variants_ident, |v| v.to_string())?;
    Ok(())
}

#[proc_macro_attribute]
pub fn check(args: TokenStream, input: TokenStream) -> TokenStream {
    let _ = args;
    let mut item_fn = parse_macro_input!(input as syn::ItemFn);

    // Run the visitor which will modify the item_fn in place
    let mut check_match = CheckMatchOrder::default();
    check_match.visit_item_fn_mut(&mut item_fn);

    // Convert AST back to a TokenStream
    let mut output: TokenStream = quote!(#item_fn).into();
    // Add the potential errors to the output stream
    for err in check_match.errors.iter() {
        output.extend::<TokenStream>(err.to_compile_error().into())
    }

    output
}

#[derive(Default)]
struct CheckMatchOrder {
    errors: Vec<syn::Error>,
}

// Implements the visitor for ExprMatch
// - Looks for #[sorted] attribute
// - Remove the attribute from the AST
// - Check for wildcard pattern and add error to the struct if not last
// - Build a vector of patterns (excluding wildcard)
// - Check the order based on the flatten path of the pattern e.g `Error::Fmt`
// - Add errors to the struct if any
impl syn::visit_mut::VisitMut for CheckMatchOrder {
    fn visit_expr_match_mut(&mut self, m: &mut syn::ExprMatch) {
        // Look for ExprMatch with `#[sorted]` attributes only
        if let Some(sorted_idx) = m.attrs.iter().position(|attr| attr.path.is_ident("sorted")) {
            // Remove `#[sorted]` from the AST
            m.attrs.remove(sorted_idx);

            // Handle wildcard pattern `_` first
            if let Some(wildcard_idx) = m
                .arms
                .iter()
                .rposition(|arm| matches!(arm.pat, syn::Pat::Wild(_)))
            {
                // Add error if the wildcard pattern is not last
                if wildcard_idx != m.arms.len() - 1 {
                    let err = syn::Error::new_spanned(
                        &m.arms[wildcard_idx].pat,
                        "Wildcard pattern `_` should be last in match",
                    );
                    self.errors.push(err);
                }
            }

            // Build an optional vector of the `syn::Path` of each pattern
            // If a pattern is not supported, we add the error and `pattern_opt` is `None`.
            let patterns_opt: Option<Vec<syn::Path>> = m
                .arms
                .iter()
                .filter(|arm| !matches!(arm.pat, syn::Pat::Wild(_)))
                .map(|arm| match &arm.pat {
                    syn::Pat::Ident(i) => Some(i.ident.clone().into()),
                    syn::Pat::Path(p) => Some(p.path.clone()),
                    syn::Pat::Struct(p) => Some(p.path.clone()),
                    syn::Pat::TupleStruct(p) => Some(p.path.clone()),
                    other => {
                        let error = syn::Error::new_spanned(other, "unsupported by #[sorted]");
                        self.errors.push(error);
                        None
                    }
                })
                .collect();

            // Check order on this vector and record error if present
            if let Some(patterns) = patterns_opt {
                let order_check = check_order(patterns, |path| {
                    path.segments
                        .iter()
                        .map(|segment| quote!(#segment).to_string())
                        .collect::<Vec<_>>()
                        .join("::")
                });

                if let Err(err) = order_check {
                    self.errors.push(err);
                };
            }
        }

        // Delegate to the default impl to visit nested expressions.
        syn::visit_mut::visit_expr_match_mut(self, m)
    }
}

// Helper checking whether the underlying Item is an Enum.
// Returns an error with span on the not-an-Enum object otherwise.
fn is_enum(item: &Item) -> Result<&syn::ItemEnum, syn::Error> {
    if let Item::Enum(item_enum) = item {
        Ok(item_enum)
    } else {
        Err(syn::Error::new_spanned(
            item,
            "#[sorted] expects an enum or a match expression",
        ))
    }
}

// Helper function that checks the order of an input vector `seq` of `T: ToTokens`
// given a function `ordered_by` that returns an `Ord` that should be used to order `T`.
// If `seq` is not ordered, the function returns a Syn::Error
// with a span matching the first element not ordered and where it should be placed.
fn check_order<T: quote::ToTokens, S: std::cmp::Ord + std::fmt::Display>(
    seq: Vec<T>,
    ordered_by: fn(&T) -> S,
) -> Result<(), syn::Error> {
    let mut names = Vec::new();
    if let Some(t) = seq.first() {
        names.push(ordered_by(t));
    }

    // Loop through consecutive pairs and compare each pair.
    // Return an error if not ordered
    for pair in seq.windows(2) {
        let name = ordered_by(&pair[0]);
        let next_name = ordered_by(&pair[1]);
        if let std::cmp::Ordering::Less = next_name.cmp(&name) {
            let next_el = names.binary_search(&next_name).unwrap_err();
            let err_msg = format!("{} should sort before {}", next_name, names[next_el]);
            return Err(syn::Error::new_spanned(&pair[1], err_msg));
        }
        names.push(next_name);
    }
    Ok(())
}
