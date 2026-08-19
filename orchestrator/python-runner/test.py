import chess
import random

number = 1

def get_chess_move(fen: str):
    global number
    """Jednoduchý šachový bot."""
    board = chess.Board(fen)
    legal_moves = list(board.generate_legal_moves())
    captures = list(board.generate_legal_captures())


    # 1. Zkusit dát mat
    for move in legal_moves:
        board.push(move)
        if board.is_checkmate():
            return move.uci()
        board.pop()
        
    # 2. Povýšení na královnu
    for move in legal_moves:
        if "q" in move.uci().lower():
            return move.uci()
            
    # 3. Braní figurky
    if captures:
        return random.choice(captures).uci()
        
    # 4. Náhodný tah
    return random.choice(legal_moves).uci()
