/// Calculate the expected LP token amount based on the token amounts and pool state
/// 
/// This follows Raydium's specified_tokens_to_lp_tokens logic:
/// lp_amount = min(
///    token0_amount * (lp_supply / token0_pool_amount),
///    token1_amount * (lp_supply / token1_pool_amount)
/// )
/// 
/// * `token0_amount` - Amount of token0 (usually WSOL)
/// * `token1_amount` - Amount of token1 (the other token)
/// * `pool_state` - The current pool state
/// * `pool_amounts` - The current pool amounts (token0, token1)
pub fn calculate_lp_amount(
    token0_amount: u64,
    token1_amount: u64,
    lp_supply: u64,
    pool_amount0: u64,
    pool_amount1: u64,
) -> Result<u64, String> {
    let lp_amount = std::cmp::min(
        ((token0_amount as u128)
            .checked_mul(lp_supply as u128)
            .and_then(|product| product.checked_div(pool_amount0 as u128))
            .and_then(|result| u64::try_from(result).ok()))
            .ok_or("Failed to calculate LP amount from token0")?,
        ((token1_amount as u128)
            .checked_mul(lp_supply as u128)
            .and_then(|product| product.checked_div(pool_amount1 as u128))
            .and_then(|result| u64::try_from(result).ok()))
            .ok_or("Failed to calculate LP amount from token1")?
    );
    
    Ok(lp_amount)
}