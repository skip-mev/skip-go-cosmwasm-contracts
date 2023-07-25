v0.1.0

v0.2.0
- Changes contract design to treat each step (fee swap, user swap, affiliate fee, post_swap_action) as separate execute_msg steps

- Deprecates
    - coin_out specification in fee swap, this will be auto derived from the ibc_fees specified.
    - coin_in specification in user swap, this will be auto derived from the coin sent to the contract minus and ibc_fees required.

- Questions:
    - How do affiliate fees work?
    - 

TODO:
Try and refactor so that I'm not passing the response and mutating it.


- So if a user wants to do swap_exact_in then we have the user_swap be the last swap so that it can use all the funds available.
- If a user wants to do swap_exact_out, then we have the user_swap be the first swap and then chain the fee swaps